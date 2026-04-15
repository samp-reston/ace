// region: Imports

use ace_proto::uds::UdsFrame;
use ace_sim::clock::{Duration, Instant};
use ace_sim::io::NodeAddress;
use ace_uds::ext::UdsFrameExt;
use ace_uds::message::service::UdsService;
use ace_uds::message::ServiceIdentifier;
use heapless::Vec;

use crate::config::{periodic, ServerConfig, SessionConfig};
use crate::handler::ServerHandler;
use crate::nrc::NrcError;
use crate::security_provider::SecurityProvider;

// endregion: Imports

// region: Capacity constants

pub const MAX_OUTBOX: usize = 16;
pub const MAX_FRAME: usize = 4096;
pub const MAX_PERIODIC: usize = 8;
pub const MAX_DATA_BUF: usize = 256;
pub const MAX_SEED: usize = 16;

// endregion: Capacity constants

// region: ServerError

#[derive(Debug)]
pub enum ServerError<E: NrcError> {
    Handler(E),
    Codec(ace_uds::error::UdsError),
    OutboxFull,
}

// endregion: ServerError

// region: SessionState

#[derive(Debug, Clone)]
struct SessionState {
    session_type: u8,
    last_rx: Instant,
    security_level: u8,
}

impl SessionState {
    fn new() -> Self {
        Self {
            session_type: 0x01,
            last_rx: Instant::ZERO,
            security_level: 0,
        }
    }

    fn is_default(&self) -> bool {
        self.session_type == 0x01
    }
}

// endregion: SessionState

// region: SecurityState

#[derive(Debug, Clone)]
struct SecurityState {
    pending_seed: Vec<u8, MAX_SEED>,
    pending_level: u8,
    failed_attempts: Vec<(u8, u8), 8>,
    lockout_until: Vec<(u8, Instant), 8>,
}

impl SecurityState {
    fn new() -> Self {
        Self {
            pending_seed: Vec::new(),
            pending_level: 0,
            failed_attempts: Vec::new(),
            lockout_until: Vec::new(),
        }
    }

    fn is_locked(&self, level: u8, now: Instant) -> bool {
        self.lockout_until
            .iter()
            .find(|(l, _)| *l == level)
            .map(|(_, until)| now < *until)
            .unwrap_or(false)
    }

    fn failed_count(&self, level: u8) -> u8 {
        self.failed_attempts
            .iter()
            .find(|(l, _)| *l == level)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    fn increment_failed(&mut self, level: u8) {
        if let Some(e) = self.failed_attempts.iter_mut().find(|(l, _)| *l == level) {
            e.1 = e.1.saturating_add(1);
        } else {
            let _ = self.failed_attempts.push((level, 1));
        }
    }

    fn reset_failed(&mut self, level: u8) {
        if let Some(e) = self.failed_attempts.iter_mut().find(|(l, _)| *l == level) {
            e.1 = 0;
        }
    }

    fn set_lockout(&mut self, level: u8, until: Instant) {
        if let Some(e) = self.lockout_until.iter_mut().find(|(l, _)| *l == level) {
            e.1 = until;
        } else {
            let _ = self.lockout_until.push((level, until));
        }
    }

    fn clear_pending(&mut self) {
        self.pending_seed.clear();
        self.pending_level = 0;
    }
}

// endregion: SecurityState

// region: PeriodicEntry / PeriodicState

#[derive(Debug, Clone)]
struct PeriodicEntry {
    did: u16,
    interval: Duration,
    next_tx: Instant,
    client: NodeAddress,
}

#[derive(Debug)]
struct PeriodicState {
    entries: Vec<PeriodicEntry, MAX_PERIODIC>,
}

impl PeriodicState {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn register(&mut self, did: u16, interval: Duration, client: NodeAddress, now: Instant) {
        if let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.did == did && e.client == client)
        {
            e.interval = interval;
            e.next_tx = now + interval;
            return;
        }
        let _ = self.entries.push(PeriodicEntry {
            did,
            interval,
            next_tx: now + interval,
            client,
        });
    }

    fn cancel(&mut self, did: u16, client: &NodeAddress) {
        self.entries
            .retain(|e| !(e.did == did && &e.client == client));
    }

    fn collect_due(&self, now: Instant, out: &mut Vec<(u16, NodeAddress), MAX_PERIODIC>) {
        for e in self.entries.iter().filter(|e| now >= e.next_tx) {
            let _ = out.push((e.did, e.client.clone()));
        }
    }

    fn advance(&mut self, did: u16, client: &NodeAddress, now: Instant) {
        if let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.did == did && &e.client == client)
        {
            e.next_tx = now + e.interval;
        }
    }
}

// endregion: PeriodicEntry / PeriodicState

// region: UdsServer

/// Stateful UDS ECU server state machine.
///
/// Receives raw UDS frames via [`handle`], uses [`UdsFrameExt`] for
/// protocol-level decisions (SID dispatch, suppress bit, sub-function),
/// and decodes typed messages only where structured field access is needed.
///
/// All timing is driven by [`tick`] - no blocking, no hardware timers,
/// no OS calls. Suitable for direct use as a `SimNode` in `ace-sim`.
pub struct UdsServer<H, S>
where
    H: ServerHandler,
    S: SecurityProvider,
{
    config: ServerConfig,
    handler: H,
    security_provider: S,
    address: NodeAddress,
    session: SessionState,
    security: SecurityState,
    periodic: PeriodicState,
    outbox: Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
}

impl<H, S> UdsServer<H, S>
where
    H: ServerHandler,
    S: SecurityProvider,
{
    pub fn new(
        config: ServerConfig,
        handler: H,
        security_provider: S,
        address: NodeAddress,
    ) -> Self {
        Self {
            config,
            handler,
            security_provider,
            address,
            session: SessionState::new(),
            security: SecurityState::new(),
            periodic: PeriodicState::new(),
            outbox: Vec::new(),
        }
    }

    // region: SimNode surface

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }
    pub fn session_type(&self) -> u8 {
        self.session.session_type
    }
    pub fn security_level(&self) -> u8 {
        self.session.security_level
    }

    /// Receives a raw UDS frame from `src`.
    ///
    /// Wraps the bytes in a [`UdsFrame`] and uses [`UdsFrameExt`] for
    /// protocol-level decisions before dispatching to service handlers.
    /// Typed message decode via `to_message()` is deferred to individual
    /// handlers that need structured field access.
    pub fn handle(
        &mut self,
        src: &NodeAddress,
        data: &[u8],
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        self.session.last_rx = now;

        let frame = UdsFrame::from_slice(data);

        // Validate frame has at minimum a SID byte
        if let Err(e) = frame.validate() {
            return Err(ServerError::Codec(e));
        }

        let sid = match frame.service_identifier() {
            Some(s) => s,
            None => return self.nrc_raw(src, 0x00, H::Error::service_not_supported().into(), now),
        };

        // Guard: service must be supported in the active session
        let sid_byte = sid.discriminant();
        if let Err(nrc) = self.guard_service(sid_byte) {
            return self.nrc_raw(src, sid_byte, nrc, now);
        }

        // Suppress bit - extracted here at the frame level before any
        // typed decode. Each handler receives this as a plain bool.
        let suppressed = frame.is_suppressed();

        match sid {
            ServiceIdentifier::UdsServiceRequest(UdsService::TesterPresentRequest) => {
                self.on_tester_present(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::DiagnosticSessionControlRequest) => {
                self.on_session_control(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::ECUResetRequest) => {
                self.on_ecu_reset(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::SecurityAccessRequest) => {
                self.on_security_access(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::ReadDataByIdentifierRequest) => {
                self.on_read_did(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::WriteDataByIdentifierRequest) => {
                self.on_write_did(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(
                UdsService::ReadDataByPeriodicIdentifierRequest,
            ) => self.on_periodic_did(src, &frame, suppressed, now),

            ServiceIdentifier::UdsServiceRequest(UdsService::RoutineControlRequest) => {
                self.on_routine_control(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::CommunicationControlRequest) => {
                self.on_communication_control(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(
                UdsService::InputOutputControlByIdentifierRequest,
            ) => self.on_io_control(src, &frame, suppressed, now),

            ServiceIdentifier::UdsServiceRequest(UdsService::RequestDownloadRequest) => {
                self.on_request_download(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::TransferDataRequest) => {
                self.on_transfer_data(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::RequestTransferExitRequest) => {
                self.on_transfer_exit(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsService::RequestFileTransferRequest) => {
                self.on_file_transfer(src, &frame, suppressed, now)
            }

            _ => self.nrc_raw(src, sid_byte, H::Error::service_not_supported().into(), now),
        }
    }

    /// Advances internal timers - S3 watchdog and periodic DID scheduling.
    pub fn tick(&mut self, now: Instant) -> Result<(), ServerError<H::Error>> {
        self.check_s3(now);
        self.dispatch_periodic(now)
    }

    /// Drains pending outbound frames into `out`.
    pub fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
    ) -> usize {
        let n = self.outbox.len();
        for item in self.outbox.drain(..) {
            let _ = out.push(item);
        }
        n
    }

    // endregion: SimNode surface

    // region: Session helpers

    fn current_session(&self) -> Option<&SessionConfig> {
        self.config.find_session(self.session.session_type)
    }

    fn check_s3(&mut self, now: Instant) {
        if self.session.is_default() {
            return;
        }
        let s3 = self
            .current_session()
            .map(|s| s.s3_timeout)
            .unwrap_or(Duration::from_millis(5_000));
        if let Some(elapsed) = now.checked_duration_since(self.session.last_rx) {
            if elapsed > s3 {
                self.session.session_type = 0x01;
                self.session.security_level = 0;
                self.security.clear_pending();
            }
        }
    }

    fn guard_service(&self, sid: u8) -> Result<(), u8> {
        if !self.config.service_allowed(sid, self.session.session_type) {
            return Err(H::Error::service_not_supported_in_active_session().into());
        }
        Ok(())
    }

    fn guard_security(&self, required: u8) -> Result<(), u8> {
        if required > 0 && self.session.security_level < required {
            return Err(H::Error::security_access_denied().into());
        }
        Ok(())
    }

    // endregion: Session helpers

    // region: Response helpers

    fn enqueue(
        &mut self,
        dst: NodeAddress,
        frame: Vec<u8, MAX_FRAME>,
    ) -> Result<(), ServerError<H::Error>> {
        self.outbox
            .push((dst, frame))
            .map_err(|_| ServerError::OutboxFull)
    }

    fn pos(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        payload: &[u8],
        _now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let mut frame: Vec<u8, MAX_FRAME> = Vec::new();
        let _ = frame.push(request_sid | 0x40);
        let _ = frame.extend_from_slice(payload);
        self.enqueue(dst.clone(), frame)
    }

    fn nrc(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        error: H::Error,
        _now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let nrc_byte: u8 = error.into();
        self.nrc_raw(dst, request_sid, nrc_byte, _now)
    }

    fn nrc_raw(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        nrc_byte: u8,
        _now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let mut frame: Vec<u8, MAX_FRAME> = Vec::new();
        let _ = frame.push(0x7F);
        let _ = frame.push(request_sid);
        let _ = frame.push(nrc_byte);
        self.enqueue(dst.clone(), frame)
    }

    // endregion: Response helpers

    // region: Service handlers
    //
    // Each handler receives:
    //   - `frame` - the raw UdsFrame for sub-function/payload access
    //   - `suppressed` - suppress bit already extracted at dispatch
    //
    // Typed decode via `frame.to_message()` is used only where structured
    // field access is needed. Services that only need the sub-function value
    // and a payload slice never allocate a typed message.

    fn on_tester_present(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // TesterPresent: sub-function is always 0x00.
        // The only meaningful information is the suppress bit.
        if suppressed {
            return Ok(());
        }
        // Echo sub-function value (suppress bit cleared) in response.
        let sf = frame.sub_function_value().unwrap_or(0x00);
        self.pos(src, 0x3E, &[sf], now)
    }

    fn on_session_control(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // Sub-function value IS the session type byte.
        let session_type = match frame.sub_function_value() {
            Some(v) => v,
            None => {
                return self.nrc(
                    src,
                    0x10,
                    H::Error::incorrect_message_length_or_invalid_format(),
                    now,
                )
            }
        };

        if self.config.find_session(session_type).is_none() {
            return self.nrc(src, 0x10, H::Error::sub_function_not_supported(), now);
        }

        self.session.session_type = session_type;
        self.session.security_level = 0;
        self.session.last_rx = now;
        self.security.clear_pending();

        if suppressed {
            return Ok(());
        }

        let (p2_ms, p2_ext_ms) = self
            .config
            .find_session(session_type)
            .map(|s| {
                (
                    s.p2_timeout.as_millis(),
                    s.p2_extended_timeout.as_millis() / 10,
                )
            })
            .unwrap_or((50, 500));

        let payload = [
            session_type,
            (p2_ms >> 8) as u8,
            p2_ms as u8,
            (p2_ext_ms >> 8) as u8,
            p2_ext_ms as u8,
        ];
        self.pos(src, 0x10, &payload, now)
    }

    fn on_ecu_reset(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // Sub-function value IS the reset type byte.
        let reset_type = match frame.sub_function_value() {
            Some(v) => v,
            None => {
                return self.nrc(
                    src,
                    0x11,
                    H::Error::incorrect_message_length_or_invalid_format(),
                    now,
                )
            }
        };

        if !suppressed {
            self.pos(src, 0x11, &[reset_type], now)?;
        }

        self.handler
            .ecu_reset(reset_type)
            .map_err(ServerError::Handler)
    }

    fn on_security_access(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool, // SecurityAccess has no suppress bit
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // Access type byte: odd = RequestSeed, even = SendKey.
        let access_type = match frame.sub_function_value() {
            Some(v) => v,
            None => {
                return self.nrc(
                    src,
                    0x27,
                    H::Error::incorrect_message_length_or_invalid_format(),
                    now,
                )
            }
        };

        let is_request_seed = access_type % 2 != 0;

        if is_request_seed {
            let level = access_type;

            if self.security.is_locked(level, now) {
                return self.nrc(src, 0x27, H::Error::required_time_delay_not_expired(), now);
            }
            if self.config.find_security_level(level).is_none() {
                return self.nrc(src, 0x27, H::Error::sub_function_not_supported(), now);
            }

            let mut seed_buf = [0u8; MAX_SEED];
            let seed_len = self
                .security_provider
                .generate_seed(level, &mut seed_buf)
                .map_err(|_| ServerError::Handler(H::Error::conditions_not_correct()))?;

            self.security.pending_seed.clear();
            let _ = self
                .security
                .pending_seed
                .extend_from_slice(&seed_buf[..seed_len]);
            self.security.pending_level = level;

            let mut payload: Vec<u8, { MAX_SEED + 1 }> = Vec::new();
            let _ = payload.push(level);
            let _ = payload.extend_from_slice(&seed_buf[..seed_len]);
            self.pos(src, 0x27, &payload, now)
        } else {
            // SendKey - key bytes are the payload after the sub-function byte.
            let level = access_type - 1; // RequestSeed level

            if self.security.is_locked(level, now) {
                return self.nrc(src, 0x27, H::Error::required_time_delay_not_expired(), now);
            }
            if self.security.pending_level != level || self.security.pending_seed.is_empty() {
                return self.nrc(src, 0x27, H::Error::request_sequence_error(), now);
            }

            // Key bytes are the payload after the access_type byte.
            // frame.payload() is everything after SID - key starts at payload[1].
            let key = frame.payload().get(1..).unwrap_or(&[]);

            let level_cfg = self.config.find_security_level(level);
            let max_attempts = level_cfg.map(|l| l.max_attempts).unwrap_or(3);
            let lockout_dur = level_cfg
                .map(|l| l.lockout_duration)
                .unwrap_or(Duration::from_millis(10_000));

            let seed = self.security.pending_seed.clone();

            match self.security_provider.validate_key(level, &seed, key) {
                Ok(()) => {
                    self.security.reset_failed(level);
                    self.security.clear_pending();
                    self.session.security_level = level;
                    self.pos(src, 0x27, &[access_type], now)
                }
                Err(_) => {
                    self.security.increment_failed(level);
                    if self.security.failed_count(level) >= max_attempts {
                        self.security.set_lockout(level, now + lockout_dur);
                        return self.nrc(src, 0x27, H::Error::exceeded_number_of_attempts(), now);
                    }
                    self.nrc(src, 0x27, H::Error::invalid_key(), now)
                }
            }
        }
    }

    fn on_read_did(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool, // ReadDataByIdentifier has no sub-function
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // Payload is pairs of DID bytes: [DID_high, DID_low, DID_high, DID_low, ...]
        let payload = frame.payload();
        if payload.len() < 2 || payload.len() % 2 != 0 {
            return self.nrc(
                src,
                0x22,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let mut resp: Vec<u8, MAX_FRAME> = Vec::new();

        for chunk in payload.chunks_exact(2) {
            let did = u16::from_be_bytes([chunk[0], chunk[1]]);

            if !self.config.did_readable(did, self.session.session_type) {
                return self.nrc(src, 0x22, H::Error::request_out_of_range(), now);
            }
            let required_sec = self
                .config
                .find_did(did)
                .map(|d| d.security_level)
                .unwrap_or(0);
            if let Err(nrc) = self.guard_security(required_sec) {
                return self.nrc_raw(src, 0x22, nrc, now);
            }

            let _ = resp.push(chunk[0]);
            let _ = resp.push(chunk[1]);

            let mut data_buf = [0u8; MAX_DATA_BUF];
            let len = self
                .handler
                .read_did(did, &mut data_buf)
                .map_err(ServerError::Handler)?;
            let _ = resp.extend_from_slice(&data_buf[..len]);
        }

        self.pos(src, 0x22, &resp, now)
    }

    fn on_write_did(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let payload = frame.payload();
        if payload.len() < 3 {
            return self.nrc(
                src,
                0x2E,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let did = u16::from_be_bytes([payload[0], payload[1]]);
        let data_rec = &payload[2..];

        if !self.config.did_writable(did, self.session.session_type) {
            return self.nrc(src, 0x2E, H::Error::request_out_of_range(), now);
        }
        let required_sec = self
            .config
            .find_did(did)
            .map(|d| d.security_level)
            .unwrap_or(0);
        if let Err(nrc) = self.guard_security(required_sec) {
            return self.nrc_raw(src, 0x2E, nrc, now);
        }

        self.handler
            .write_did(did, data_rec)
            .map_err(ServerError::Handler)?;

        self.pos(src, 0x2E, &payload[..2], now)
    }

    fn on_periodic_did(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let payload = frame.payload();
        if payload.is_empty() {
            return self.nrc(
                src,
                0x2A,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        // Byte 0 is transmission mode, remaining bytes are periodic DID identifiers.
        let mode = payload[0];
        let periodic_ids = payload.get(1..).unwrap_or(&[]);

        match mode {
            // stopSending
            0x04 => {
                for &id in periodic_ids {
                    self.periodic.cancel(0xF200u16 | id as u16, src);
                }
                self.pos(src, 0x2A, &[mode], now)
            }
            0x01 | 0x02 | 0x03 => {
                let requested_interval = match mode {
                    0x01 => periodic::SLOW,
                    0x02 => periodic::MEDIUM,
                    _ => periodic::FAST,
                };

                for &id in periodic_ids {
                    let did = 0xF200u16 | id as u16;

                    if !self.config.did_readable(did, self.session.session_type) {
                        return self.nrc(src, 0x2A, H::Error::request_out_of_range(), now);
                    }

                    let effective = self
                        .config
                        .find_did(did)
                        .map(|d| requested_interval.max(d.min_periodic_interval))
                        .unwrap_or(requested_interval);

                    self.periodic.register(did, effective, src.clone(), now);
                }

                self.pos(src, 0x2A, &[mode], now)
            }
            _ => self.nrc(src, 0x2A, H::Error::sub_function_not_supported(), now),
        }
    }

    fn on_routine_control(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // Sub-function value is the routine control type (0x01/0x02/0x03).
        // Payload after SID: [sub_function, routine_id_high, routine_id_low, option_record...]
        let payload = frame.payload();
        if payload.len() < 3 {
            return self.nrc(
                src,
                0x31,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let sub_function = frame.sub_function_value().unwrap_or(0);
        let routine_id = u16::from_be_bytes([payload[1], payload[2]]);
        let option_record = payload.get(3..).unwrap_or(&[]);

        let mut buf = [0u8; MAX_DATA_BUF];
        let len = self
            .handler
            .routine_control(routine_id, sub_function, option_record, &mut buf)
            .map_err(ServerError::Handler)?;

        if suppressed {
            return Ok(());
        }

        let mut resp: Vec<u8, { MAX_DATA_BUF + 3 }> = Vec::new();
        let _ = resp.push(sub_function);
        let _ = resp.push(payload[1]);
        let _ = resp.push(payload[2]);
        let _ = resp.extend_from_slice(&buf[..len]);
        self.pos(src, 0x31, &resp, now)
    }

    fn on_communication_control(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let payload = frame.payload();
        if payload.len() < 2 {
            return self.nrc(
                src,
                0x28,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let control_type = frame.sub_function_value().unwrap_or(0);
        let comm_type = payload[1];

        self.handler
            .communication_control(control_type, comm_type)
            .map_err(ServerError::Handler)?;

        if suppressed {
            return Ok(());
        }

        self.pos(src, 0x28, &[control_type], now)
    }

    fn on_io_control(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let payload = frame.payload();
        if payload.len() < 3 {
            return self.nrc(
                src,
                0x2F,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let did = u16::from_be_bytes([payload[0], payload[1]]);
        let control_param = payload[2];
        let control_state = payload.get(3..).unwrap_or(&[]);

        let mut buf = [0u8; MAX_DATA_BUF];
        let len = self
            .handler
            .io_control(did, control_param, control_state, &mut buf)
            .map_err(ServerError::Handler)?;

        let mut resp: Vec<u8, { MAX_DATA_BUF + 2 }> = Vec::new();
        let _ = resp.push(payload[0]);
        let _ = resp.push(payload[1]);
        let _ = resp.extend_from_slice(&buf[..len]);
        self.pos(src, 0x2F, &resp, now)
    }

    fn on_request_download(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        // Payload: [data_format_identifier, address_and_length_format,
        //           memory_address..., memory_size...]
        let payload = frame.payload();
        if payload.len() < 3 {
            return self.nrc(
                src,
                0x34,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let data_format = payload[0];
        let addr_and_len_format = payload[1];
        let addr_len = (addr_and_len_format >> 4) as usize;
        let size_len = (addr_and_len_format & 0x0F) as usize;

        if payload.len() < 2 + addr_len + size_len {
            return self.nrc(
                src,
                0x34,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let memory_address = &payload[2..2 + addr_len];
        let memory_size = &payload[2 + addr_len..2 + addr_len + size_len];

        let mut buf = [0u8; 64];
        let len = self
            .handler
            .request_download(memory_address, memory_size, data_format, 0, &mut buf)
            .map_err(ServerError::Handler)?;

        self.pos(src, 0x34, &buf[..len], now)
    }

    fn on_transfer_data(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let payload = frame.payload();
        if payload.is_empty() {
            return self.nrc(
                src,
                0x36,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let block_seq = payload[0];
        let data = payload.get(1..).unwrap_or(&[]);

        let mut buf = [0u8; MAX_DATA_BUF];
        let len = self
            .handler
            .transfer_data(block_seq, data, &mut buf)
            .map_err(ServerError::Handler)?;

        let mut resp: Vec<u8, { MAX_DATA_BUF + 1 }> = Vec::new();
        let _ = resp.push(block_seq);
        let _ = resp.extend_from_slice(&buf[..len]);
        self.pos(src, 0x36, &resp, now)
    }

    fn on_transfer_exit(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let parameter_record = frame.payload();

        let mut buf = [0u8; MAX_DATA_BUF];
        let len = self
            .handler
            .request_transfer_exit(parameter_record, &mut buf)
            .map_err(ServerError::Handler)?;

        self.pos(src, 0x37, &buf[..len], now)
    }

    fn on_file_transfer(
        &mut self,
        src: &NodeAddress,
        frame: &UdsFrame<'_>,
        _suppressed: bool,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let payload = frame.payload();
        if payload.len() < 3 {
            return self.nrc(
                src,
                0x38,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let operation = payload[0];
        // Bytes 1-2 are file path length (big-endian u16), rest is path
        let path_len = u16::from_be_bytes([payload[1], payload[2]]) as usize;
        let path = payload.get(3..3 + path_len).unwrap_or(&[]);

        let mut buf = [0u8; MAX_DATA_BUF];
        let len = self
            .handler
            .request_file_transfer(operation, path, &mut buf)
            .map_err(ServerError::Handler)?;

        self.pos(src, 0x38, &buf[..len], now)
    }

    // endregion: Service handlers

    // region: Periodic dispatch

    fn dispatch_periodic(&mut self, now: Instant) -> Result<(), ServerError<H::Error>> {
        let mut due: Vec<(u16, NodeAddress), MAX_PERIODIC> = Vec::new();
        self.periodic.collect_due(now, &mut due);

        for (did, client) in &due {
            let mut data_buf = [0u8; MAX_DATA_BUF];
            let len = self
                .handler
                .read_did(*did, &mut data_buf)
                .map_err(ServerError::Handler)?;

            // [periodic_data_identifier (1 byte), data_record (n bytes)]
            let did_low = (*did & 0xFF) as u8;
            let mut frame = Vec::new();
            let _ = frame.push(did_low);
            let _ = frame.extend_from_slice(&data_buf[..len]);

            self.enqueue(client.clone(), frame)?;
            self.periodic.advance(*did, client, now);
        }

        Ok(())
    }

    // endregion: Periodic dispatch
}

// endregion: UdsServer
