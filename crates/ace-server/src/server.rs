// region: Imports

use ace_core::AddressMode;
use ace_core::Vec;
use ace_core::clock::{Duration, Instant};
use ace_core::io::{InboundFrame, NodeAddress};
use ace_core::storage::CapacityError;
use ace_proto::uds::UdsFrame;
use ace_uds::ext::UdsFrameExt;
use ace_uds::message::ServiceIdentifier;
use ace_uds::message::service::UdsServiceRequest;

use crate::config::{ServerConfig, SessionConfig, periodic};
use crate::handler::ServerHandler;
use crate::nrc::NrcError;
use crate::security_provider::SecurityProvider;

// endregion: Imports

// region: ServerError

#[derive(Debug)]
pub enum ServerError<E: NrcError> {
    Handler(E),

    /// A pending request was completed when none was outstanding.
    ///
    /// Reported rather than ignored: a caller answering a request the server is not waiting
    /// on has lost track of its own work, and silently dropping the response would leave the
    /// real request to expire instead.
    NoPendingRequest,

    Codec(ace_uds::error::UdsError),
    OutboxFull,

    /// A response did not fit its frame buffer on a path with no request to answer.
    ///
    /// Request/response paths report this condition to the tester as NRC 0x14
    /// instead. This variant is for unsolicited transmissions - periodic data -
    /// where there is no NRC channel, so the only honest option is to tell the
    /// caller rather than send a malformed frame.
    ResponseTooLong,
}

// endregion: ServerError

// region: PendingRequest

/// A request answered with `0x78` and awaiting its final response.
#[derive(Debug, Clone)]
struct PendingRequest {
    /// Who to answer. Kept because the final response must reach the original requester
    /// even though no frame is in hand when it is produced.
    dst: NodeAddress,

    /// The SID being answered, so the final response carries the right one.
    sid: u8,

    /// When another `0x78` is due if the work has still not finished.
    deadline: Instant,
}

// endregion: PendingRequest

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
/// Per-level security bookkeeping.
///
/// Both tables are sized by `MAX_SECURITY_LEVELS`, the same bound the consumer
/// already gives `ServerConfig`. They were previously fixed at 8 regardless, so a
/// consumer who correctly declared more levels than that got a server whose runtime
/// state could not track them all - ISO 14229-1 permits far more than 8. Because
/// `ServerConfig::security_levels` is itself bounded by `MAX_SECURITY_LEVELS`, these
/// can now always hold an entry for every level the consumer configured.
struct SecurityState<const MAX_SEED: usize, const MAX_SECURITY_LEVELS: usize> {
    pending_seed: Vec<u8, MAX_SEED>,
    pending_level: u8,
    failed_attempts: Vec<(u8, u8), MAX_SECURITY_LEVELS>,
    lockout_until: Vec<(u8, Instant), MAX_SECURITY_LEVELS>,
}

impl<const MAX_SEED: usize, const MAX_SECURITY_LEVELS: usize>
    SecurityState<MAX_SEED, MAX_SECURITY_LEVELS>
{
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

    /// Records a failed attempt for `level`.
    ///
    /// Fallible on purpose. `failed_count` returns 0 for a level it is not tracking,
    /// so an unrecorded failure means the attempt limit never trips and the level
    /// accepts unlimited key guesses. The caller must treat `Err` as a denial.
    fn increment_failed(&mut self, level: u8) -> Result<(), CapacityError> {
        if let Some(e) = self.failed_attempts.iter_mut().find(|(l, _)| *l == level) {
            e.1 = e.1.saturating_add(1);
            Ok(())
        } else {
            self.failed_attempts
                .push((level, 1))
                .map_err(|_| CapacityError)
        }
    }

    fn reset_failed(&mut self, level: u8) {
        if let Some(e) = self.failed_attempts.iter_mut().find(|(l, _)| *l == level) {
            e.1 = 0;
        }
    }

    /// Records a lockout for `level` until `until`.
    ///
    /// Fallible on purpose. `is_locked` returns false for a level it is not tracking,
    /// so an unrecorded lockout is never enforced. The caller must treat `Err` as a
    /// denial rather than proceeding as though the lockout were in place.
    fn set_lockout(&mut self, level: u8, until: Instant) -> Result<(), CapacityError> {
        if let Some(e) = self.lockout_until.iter_mut().find(|(l, _)| *l == level) {
            e.1 = until;
            Ok(())
        } else {
            self.lockout_until
                .push((level, until))
                .map_err(|_| CapacityError)
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
struct PeriodicState<const MAX_PERIODIC: usize> {
    entries: Vec<PeriodicEntry, MAX_PERIODIC>,
}

impl<const MAX_PERIODIC: usize> PeriodicState<MAX_PERIODIC> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn register(
        &mut self,
        did: u16,
        interval: Duration,
        client: NodeAddress,
        now: Instant,
    ) -> Result<(), CapacityError> {
        if let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.did == did && e.client == client)
        {
            e.interval = interval;
            e.next_tx = now + interval;
            return Ok(());
        }

        // Reported rather than dropped: silently failing to register leaves the client
        // with a positive response and no periodic data ever arriving. ISO 14229-1
        // expects a negative response when the server cannot accept the subscription.
        self.entries
            .push(PeriodicEntry {
                did,
                interval,
                next_tx: now + interval,
                client,
            })
            .map_err(|_| CapacityError)
    }

    fn cancel(&mut self, did: u16, client: &NodeAddress) {
        self.entries
            .retain(|e| !(e.did == did && &e.client == client));
    }

    /// Collects entries whose next transmission is due into `out`.
    ///
    /// Infallible by construction: `out` and `self.entries` share the `MAX_PERIODIC`
    /// bound and the result is a subset of `entries`, so a caller passing an empty
    /// `out` cannot overflow it. The pushes below rely on that.
    fn collect_due(&self, now: Instant, out: &mut Vec<(u16, NodeAddress), MAX_PERIODIC>) {
        debug_assert!(out.is_empty(), "collect_due expects an empty output buffer");
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
#[derive(Debug)]
pub struct UdsServer<
    const MAX_FRAME: usize,
    const MAX_OUTBOX: usize,
    const MAX_SESSIONS: usize,
    const MAX_SERVICES: usize,
    const MAX_DIDS: usize,
    const MAX_SECURITY_LEVELS: usize,
    const DEFAULT_S3: u64,
    const DEFAULT_P2: u64,
    const DEFAULT_P2_EXT: u64,
    const DEFAULT_LOCKOUT: u64,
    const DEFAULT_MAX_SECURITY_ATTEMPTS: u8,
    const MAX_SEED: usize,
    const MAX_PERIODIC: usize,
    H,
    S,
> where
    H: ServerHandler,
    S: SecurityProvider,
{
    config: ServerConfig<MAX_SESSIONS, MAX_SERVICES, MAX_DIDS, MAX_SECURITY_LEVELS>,
    handler: H,
    security_provider: S,
    address: NodeAddress,
    session: SessionState,
    security: SecurityState<MAX_SEED, MAX_SECURITY_LEVELS>,
    periodic: PeriodicState<MAX_PERIODIC>,

    /// The request this server has answered with `0x78` and not yet finished.
    ///
    /// ISO 14229-1 makes the final response mandatory once `0x78` has been sent, so this is
    /// state rather than a predicate: the obligation outlives the request that created it and
    /// cannot be recomputed from a later frame.
    pending: Option<PendingRequest>,

    /// How the request currently being handled was addressed.
    ///
    /// Per-request and transient: set at the top of [`Self::handle`] and read by
    /// [`Self::nrc_raw`] to decide whether ISO 14229-1's functional suppression
    /// applies. Defaults to `Physical` so any path that emits an NRC outside a
    /// `handle` call answers rather than falling silent.
    inbound: AddressMode,
    outbox: Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
}

impl<
    const MAX_FRAME: usize,
    const MAX_OUTBOX: usize,
    const MAX_SESSIONS: usize,
    const MAX_SERVICES: usize,
    const MAX_DIDS: usize,
    const MAX_SECURITY_LEVELS: usize,
    const DEFAULT_S3: u64,
    const DEFAULT_P2: u64,
    const DEFAULT_P2_EXT: u64,
    const DEFAULT_LOCKOUT: u64,
    const DEFAULT_MAX_SECURITY_ATTEMPTS: u8,
    const MAX_SEED: usize,
    const MAX_PERIODIC: usize,
    H,
    S,
>
    UdsServer<
        MAX_FRAME,
        MAX_OUTBOX,
        MAX_SESSIONS,
        MAX_SERVICES,
        MAX_DIDS,
        MAX_SECURITY_LEVELS,
        DEFAULT_S3,
        DEFAULT_P2,
        DEFAULT_P2_EXT,
        DEFAULT_LOCKOUT,
        DEFAULT_MAX_SECURITY_ATTEMPTS,
        MAX_SEED,
        MAX_PERIODIC,
        H,
        S,
    >
where
    H: ServerHandler,
    S: SecurityProvider,
{
    pub fn new(
        config: ServerConfig<MAX_SESSIONS, MAX_SERVICES, MAX_DIDS, MAX_SECURITY_LEVELS>,
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
            pending: None,
            inbound: AddressMode::Physical,
            outbox: Vec::new(),
        }
    }

    // region: SimNode surface

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }
    /// The UDS logical address this server answers on.
    pub fn physical_address(&self) -> u16 {
        self.config.physical_address
    }

    /// The UDS functional (broadcast) address this server listens on.
    ///
    /// A transport uses this to translate its own broadcast identifier - CAN ID 0x7DF
    /// conventionally - into the UDS address space before delivery, so this server
    /// never has to know what a CAN ID is.
    pub fn functional_address(&self) -> u16 {
        self.config.functional_address
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
        frame: &InboundFrame<'_>,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        match self.dispatch_inbound(frame, now) {
            Err(ServerError::Handler(error)) => {
                // A handler's error *is* a negative response code - that is what the
                // `NrcError` bound on `ServerHandler::Error` means. Every one of them used
                // to escape to the caller instead, so a handler refusing a request produced
                // no answer at all: the client waited out P2 for a response the server had
                // already decided not to send, and whatever drove the server saw a Rust
                // error it could do nothing useful with.
                //
                // Routing them through `nrc` is also what makes `response_pending`
                // reachable, since that is where `0x78` is recognised.
                let request_sid = frame.data.first().copied().unwrap_or(0);
                let requester = frame.src.clone();

                self.nrc(&requester, request_sid, error, now)
            }
            other => other,
        }
    }

    fn dispatch_inbound(
        &mut self,
        frame: &InboundFrame<'_>,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let src = &frame.src;
        let data = frame.data;

        // Ownership filter. `dst` is a UDS logical address: the transport has already
        // mapped its own addressing onto this space, so a CAN node compares logical
        // addresses here and never sees a CAN ID.
        //
        // A frame addressed to neither of this server's addresses is not ours. Staying
        // silent is the whole point - before this existed the filtering lived in
        // whatever drove the node, so a real ECU inherited none of it.
        self.inbound = if frame.dst.0 == u32::from(self.config.physical_address) {
            AddressMode::Physical
        } else if frame.dst.0 == u32::from(self.config.functional_address) {
            AddressMode::Functional
        } else {
            return Ok(());
        };

        self.session.last_rx = now;

        // UDS allows one request at a time on a physical connection. Accepting a second
        // while a response is still pending would leave two requests competing for one
        // pending slot, and the first - already promised an answer - would never get one.
        if self.pending.is_some() {
            let sid = data.first().copied().unwrap_or(0);
            return self.nrc_raw_unconditional(
                &frame.dst.clone(),
                sid,
                H::Error::busy_repeat_request().into(),
                now,
            );
        }

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
            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::TesterPresent) => {
                self.on_tester_present(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::DiagnosticSessionControl) => {
                self.on_session_control(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::EcuReset) => {
                self.on_ecu_reset(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::SecurityAccess) => {
                self.on_security_access(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::ReadDataByIdentifier) => {
                self.on_read_did(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::WriteDataByIdentifier) => {
                self.on_write_did(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(
                UdsServiceRequest::ReadDataByPeriodicIdentifier,
            ) => self.on_periodic_did(src, &frame, suppressed, now),

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RoutineControl) => {
                self.on_routine_control(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::CommunicationControl) => {
                self.on_communication_control(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(
                UdsServiceRequest::InputOutputControlByIdentifier,
            ) => self.on_io_control(src, &frame, suppressed, now),

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestDownload) => {
                self.on_request_download(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::TransferData) => {
                self.on_transfer_data(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestTransferExit) => {
                self.on_transfer_exit(src, &frame, suppressed, now)
            }

            ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestFileTransfer) => {
                self.on_file_transfer(src, &frame, suppressed, now)
            }

            _ => self.nrc_raw(src, sid_byte, H::Error::service_not_supported().into(), now),
        }
    }

    /// Advances internal timers - S3 watchdog and periodic DID scheduling.
    pub fn tick(&mut self, now: Instant) -> Result<(), ServerError<H::Error>> {
        self.check_s3(now);
        self.refresh_pending(now)?;
        self.dispatch_periodic(now)
    }

    /// Re-sends `0x78` when the P2* budget for a pending request runs out.
    ///
    /// ISO 14229-1 permits repeated response-pending messages, each restarting P2*. Without
    /// this the client's extended deadline would expire while the server was still working,
    /// and the client would declare a timeout on a request the server fully intends to
    /// answer - the exact confusion `0x78` exists to prevent.
    fn refresh_pending(&mut self, now: Instant) -> Result<(), ServerError<H::Error>> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(());
        };

        if now < pending.deadline {
            return Ok(());
        }

        let dst = pending.dst.clone();
        let sid = pending.sid;
        let next_deadline = now + self.p2_extended();

        if let Some(pending) = self.pending.as_mut() {
            pending.deadline = next_deadline;
        }

        self.nrc_raw_unconditional(&dst, sid, H::Error::response_pending().into(), now)
    }

    /// Drains pending outbound frames into `out`.
    pub fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
    ) -> Result<usize, CapacityError> {
        // Refused before draining rather than during: the source is emptied as it is
        // read, so a partial move would lose the remainder with no way to recover it.
        if self.outbox.len() > out.capacity() - out.len() {
            return Err(CapacityError);
        }

        let n = self.outbox.len();

        for item in self.outbox.drain(..) {
            // Cannot fail: capacity checked above.
            let _ = out.push(item);
        }

        Ok(n)
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
            .unwrap_or(Duration::from_millis(DEFAULT_S3));
        if let Some(elapsed) = now.checked_duration_since(self.session.last_rx)
            && elapsed > s3
        {
            self.session.session_type = 0x01;
            self.session.security_level = 0;
            self.security.clear_pending();
        }
    }

    /// Rejects a service the session does not allow.
    ///
    /// Distinguishes two conditions the standard gives different codes: a SID this
    /// server does not implement at all is `serviceNotSupported`, while one it
    /// implements but which this session does not permit is
    /// `serviceNotSupportedInActiveSession`. Collapsing both into the latter, as this
    /// did previously, tells a tester the service might work in another session when it
    /// does not exist here at all.
    ///
    /// Both codes are suppressed on a functionally addressed request, so this changes
    /// only what a physically addressed tester is told - accurately.
    fn guard_service(&self, sid: u8) -> Result<(), u8> {
        if self.config.find_service(sid).is_none() {
            return Err(H::Error::service_not_supported().into());
        }

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
        // Propagated from the push itself. This replaces a `len() >= MAX_OUTBOX`
        // guard that existed only because `push` had different signatures in the two
        // feature configurations; the Phase 3 storage newtype removed that split.
        self.outbox
            .push((dst, frame))
            .map_err(|_| ServerError::OutboxFull)?;

        Ok(())
    }

    fn pos(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        payload: &[u8],
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let mut frame: Vec<u8, MAX_FRAME> = Vec::new();

        // Cannot fail: a server configured with MAX_FRAME < 1 cannot answer anything.
        let _ = frame.push(request_sid | 0x40);

        if frame.extend_from_slice(payload).is_err() {
            // ISO 14229-1 0x14 responseTooLong. The payload plus the response SID
            // byte exceeds MAX_FRAME. Truncating instead would put a well-formed
            // looking but semantically wrong frame on the wire, and still report
            // success - the tester would decode a positive response with its data
            // field silently missing.
            //
            // Note the frame needs `payload.len() + 1`, so a handler that fills a
            // MAX_FRAME-sized response buffer completely will land here: the
            // response SID leaves only MAX_FRAME - 1 bytes for data.
            return self.nrc_raw(dst, request_sid, H::Error::response_too_long().into(), now);
        }

        self.enqueue(dst.clone(), frame)
    }

    /// Sends NRC 0x14 responseTooLong for a request whose response would not fit.
    ///
    /// Used by the handler paths that assemble a response body before handing it to
    /// [`Self::pos`], where an over-long handler result would otherwise be lost
    /// before `pos` ever sees it.
    fn nrc_too_long(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        self.nrc_raw(dst, request_sid, H::Error::response_too_long().into(), now)
    }

    fn nrc(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        error: H::Error,
        _now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let nrc_byte: u8 = error.into();

        // `0x78` is not a refusal, it is a promise. A handler returning it says the request
        // was accepted and is still being worked on, so instead of terminating the exchange
        // the server records the obligation to finish it.
        if nrc_byte == H::Error::response_pending().into() {
            return self.begin_pending(dst, request_sid, _now);
        }

        self.nrc_raw(dst, request_sid, nrc_byte, _now)
    }

    /// The P2* budget for the active session, defaulting to this server's configured value.
    fn p2_extended(&self) -> Duration {
        self.config
            .find_session(self.session.session_type)
            .map(|s| s.p2_extended_timeout)
            .unwrap_or(Duration::from_millis(DEFAULT_P2_EXT))
    }

    /// Records that a request is still being worked on and tells the client so.
    fn begin_pending(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        self.pending = Some(PendingRequest {
            dst: dst.clone(),
            sid: request_sid,
            deadline: now + self.p2_extended(),
        });

        // Sent unconditionally. ISO 14229-1: once this NRC is used the server shall always
        // send a final response, independent of the suppress-positive bit and of the
        // suppression required for the five "not implemented" codes on a functionally
        // addressed request. A promise that could itself be suppressed would be worthless.
        self.nrc_raw_unconditional(dst, request_sid, H::Error::response_pending().into(), now)
    }

    /// Whether a request is awaiting its final response.
    pub fn has_pending_request(&self) -> bool {
        self.pending.is_some()
    }

    /// Answers a pending request positively with `payload`.
    ///
    /// The counterpart to a handler returning `response_pending`: the long-running work
    /// finishes outside the request that started it and hands its result back here. The
    /// response is sent regardless of how the original request was addressed or whether it
    /// asked for positive responses to be suppressed, because `0x78` has already committed
    /// the server to answering.
    pub fn complete_pending(
        &mut self,
        payload: &[u8],
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let Some(pending) = self.pending.take() else {
            return Err(ServerError::NoPendingRequest);
        };

        self.pos(&pending.dst, pending.sid, payload, now)
    }

    /// Answers a pending request negatively.
    ///
    /// For work that was accepted and then failed. Also unconditional - the client is owed
    /// an answer either way, and a failure it never hears about is indistinguishable to it
    /// from a server that stopped responding.
    pub fn fail_pending(
        &mut self,
        error: H::Error,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let Some(pending) = self.pending.take() else {
            return Err(ServerError::NoPendingRequest);
        };

        self.nrc_raw_unconditional(&pending.dst, pending.sid, error.into(), now)
    }

    /// NRCs ISO 14229-1 requires a server to withhold on a functionally addressed
    /// request: serviceNotSupported, subFunctionNotSupported, requestOutOfRange,
    /// subFunctionNotSupportedInActiveSession, serviceNotSupportedInActiveSession.
    ///
    /// Every one means "this server does not implement that". On a broadcast most
    /// servers legitimately do not, so answering would flood the client.
    const FUNCTIONALLY_SUPPRESSED: [u8; 5] = [0x11, 0x12, 0x31, 0x7E, 0x7F];

    fn nrc_raw(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        nrc_byte: u8,
        now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        if self.inbound == AddressMode::Functional
            && Self::FUNCTIONALLY_SUPPRESSED.contains(&nrc_byte)
        {
            return Ok(());
        }

        self.nrc_raw_unconditional(dst, request_sid, nrc_byte, now)
    }

    /// Sends a negative response regardless of how the request was addressed.
    ///
    /// For the case ISO 14229-1 distinguishes from "not implemented": the request *is*
    /// ours and processing it failed. Such a response is transmitted even on a
    /// functional request, and even with the suppress-positive bit set, because the
    /// condition is specific to this server and the client needs it.
    fn nrc_raw_unconditional(
        &mut self,
        dst: &NodeAddress,
        request_sid: u8,
        nrc_byte: u8,
        _now: Instant,
    ) -> Result<(), ServerError<H::Error>> {
        let mut frame: Vec<u8, MAX_FRAME> = Vec::new();

        // A negative response is exactly three bytes. These cannot fail for any
        // server that can answer at all, and they deliberately do not recurse into
        // the responseTooLong path: failing to build an NRC has no NRC to report it.
        debug_assert!(
            MAX_FRAME >= 3,
            "MAX_FRAME must hold a 3-byte negative response"
        );
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
                );
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
            // Both branches must be in the units the response encodes: P2 in milliseconds
            // and P2* in tens of milliseconds. The fallback previously used the configured
            // value verbatim while the session branch divided by ten, so a server with no
            // matching session config advertised a P2* ten times longer than it honours -
            // and a client that believed it would wait ten times too long before giving up.
            .unwrap_or((DEFAULT_P2, DEFAULT_P2_EXT / 10));

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
                );
            }
        };

        // Attempted before it is confirmed. This previously sent the positive response
        // first, so a reset the handler then refused had already been reported as
        // successful - and with handler errors now reaching the client, it would have
        // produced a positive response and a negative one for the same request.
        self.handler
            .ecu_reset(reset_type)
            .map_err(ServerError::Handler)?;

        if !suppressed {
            self.pos(src, 0x11, &[reset_type], now)?;
        }

        // A reset returns the server to its power-on diagnostic state. Previously it
        // reset nothing: the session, the unlocked security level and any pending seed all
        // survived, so a tester could reset an ECU and still be in a programming session
        // holding an unlocked level - a privilege that should not outlive the reset.
        //
        // Ordered after the response so the answer reflects the request that was accepted.
        self.session.session_type = 0x01;
        self.session.security_level = 0;
        self.session.last_rx = now;
        self.security.clear_pending();

        // Lockouts deliberately survive. Clearing them would make a reset an escape from
        // the attempt limit, so an attacker could reset between guesses and never be
        // delayed - which is what the limit exists to prevent.

        Ok(())
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
                );
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

            // `SecurityLevelConfig::seed_length` was declared and read nowhere: the server
            // handed out whatever the provider produced. A provider returning a different
            // length than the level declares is a contract violation, and shipping the
            // wrong-length seed would make the tester derive a key against it and fail the
            // unlock with nothing to explain why.
            //
            // Reported as conditionsNotCorrect, matching the `generate_seed` failure path
            // above: the fault is the server's, not a malformed request.
            if let Some(expected) = self
                .config
                .find_security_level(level)
                .map(|l| l.seed_length)
                && seed_len != expected
            {
                self.security.clear_pending();
                return self.nrc(src, 0x27, H::Error::conditions_not_correct(), now);
            }

            self.security.pending_seed.clear();
            // A truncated stored seed would make the later SendKey comparison run
            // against the wrong seed, so a legitimate unlock would fail with no
            // indication why.
            if self
                .security
                .pending_seed
                .extend_from_slice(&seed_buf[..seed_len])
                .is_err()
            {
                self.security.pending_seed.clear();
                return self.nrc_too_long(src, 0x27, now);
            }
            self.security.pending_level = level;

            let mut payload: Vec<u8, MAX_SEED> = Vec::new();
            if payload.push(level).is_err()
                || payload.extend_from_slice(&seed_buf[..seed_len]).is_err()
            {
                self.security.pending_seed.clear();
                self.security.pending_level = 0;
                return self.nrc_too_long(src, 0x27, now);
            }
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
            let max_attempts = level_cfg
                .map(|l| l.max_attempts)
                .unwrap_or(DEFAULT_MAX_SECURITY_ATTEMPTS);
            let lockout_dur = level_cfg
                .map(|l| l.lockout_duration)
                .unwrap_or(Duration::from_millis(DEFAULT_LOCKOUT));

            if let Some(expected) = level_cfg.map(|l| l.key_length)
                && key.len() != expected
            {
                return self.nrc(
                    src,
                    0x27,
                    H::Error::incorrect_message_length_or_invalid_format(),
                    now,
                );
            }

            let seed = self.security.pending_seed.clone();

            match self.security_provider.validate_key(level, &seed, key) {
                Ok(()) => {
                    self.security.reset_failed(level);
                    self.security.clear_pending();
                    self.session.security_level = level;
                    self.pos(src, 0x27, &[access_type], now)
                }
                Err(_) => {
                    // A failed key invalidates the seed. Without this the client could
                    // send key after key against the same seed, which is the reuse the
                    // attempt counter exists to bound - and every retry would be measured
                    // against a seed the server had already shown it would not accept.
                    // ISO 14229-1 requires a fresh requestSeed after a failure.
                    self.security.clear_pending();

                    // Fail closed. If the failed attempt cannot be recorded then the
                    // attempt limit cannot be enforced, and proceeding would grant
                    // this level unlimited key guesses. Deny instead of allowing an
                    // unbounded retry window.
                    if self.security.increment_failed(level).is_err() {
                        return self.nrc(src, 0x27, H::Error::conditions_not_correct(), now);
                    }

                    if self.security.failed_count(level) >= max_attempts {
                        // Likewise: an unrecorded lockout is never enforced, so a
                        // failure here must deny rather than report a lockout that
                        // does not exist.
                        if self.security.set_lockout(level, now + lockout_dur).is_err() {
                            return self.nrc(src, 0x27, H::Error::conditions_not_correct(), now);
                        }

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
        if payload.len() < 2 || !payload.len().is_multiple_of(2) {
            return self.nrc(
                src,
                0x22,
                H::Error::incorrect_message_length_or_invalid_format(),
                now,
            );
        }

        let mut resp: Vec<u8, MAX_FRAME> = Vec::new();

        for chunk in payload.as_chunks::<2>().0 {
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

            if resp.push(chunk[0]).is_err() || resp.push(chunk[1]).is_err() {
                return self.nrc_too_long(src, 0x22, now);
            }

            let mut data_buf = [0u8; MAX_FRAME];
            let len = self
                .handler
                .read_did(did, &mut data_buf)
                .map_err(ServerError::Handler)?;
            if resp.extend_from_slice(&data_buf[..len]).is_err() {
                return self.nrc_too_long(src, 0x22, now);
            }
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
            0x01..=0x03 => {
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

                    // Refuse rather than confirm a subscription that was not stored:
                    // a positive response here would promise periodic data the server
                    // has no record of and will never send.
                    //
                    // Sent unconditionally. This reuses requestOutOfRange, which is
                    // normally suppressed on a functional request, but the meaning here
                    // is the opposite of "not implemented": the DID *is* supported and
                    // the server ran out of room. ISO 14229-1 requires that outcome to
                    // be reported even on a broadcast, so it must bypass the
                    // suppression.
                    if self
                        .periodic
                        .register(did, effective, src.clone(), now)
                        .is_err()
                    {
                        return self.nrc_raw_unconditional(
                            src,
                            0x2A,
                            H::Error::request_out_of_range().into(),
                            now,
                        );
                    }
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

        let mut buf = [0u8; MAX_FRAME];
        let len = self
            .handler
            .routine_control(routine_id, sub_function, option_record, &mut buf)
            .map_err(ServerError::Handler)?;

        if suppressed {
            return Ok(());
        }

        let mut resp: Vec<u8, MAX_FRAME> = Vec::new();
        if resp.push(sub_function).is_err()
            || resp.push(payload[1]).is_err()
            || resp.push(payload[2]).is_err()
            || resp.extend_from_slice(&buf[..len]).is_err()
        {
            return self.nrc_too_long(src, 0x31, now);
        }
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

        let mut buf = [0u8; MAX_FRAME];
        let len = self
            .handler
            .io_control(did, control_param, control_state, &mut buf)
            .map_err(ServerError::Handler)?;

        let mut resp: Vec<u8, MAX_FRAME> = Vec::new();
        if resp.push(payload[0]).is_err()
            || resp.push(payload[1]).is_err()
            || resp.extend_from_slice(&buf[..len]).is_err()
        {
            return self.nrc_too_long(src, 0x2F, now);
        }
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

        let mut buf = [0u8; MAX_FRAME];
        let len = self
            .handler
            .transfer_data(block_seq, data, &mut buf)
            .map_err(ServerError::Handler)?;

        let mut resp: Vec<u8, MAX_FRAME> = Vec::new();
        if resp.push(block_seq).is_err() || resp.extend_from_slice(&buf[..len]).is_err() {
            return self.nrc_too_long(src, 0x36, now);
        }
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

        let mut buf = [0u8; MAX_FRAME];
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

        let mut buf = [0u8; MAX_FRAME];
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
            let mut data_buf = [0u8; MAX_FRAME];
            let len = self
                .handler
                .read_did(*did, &mut data_buf)
                .map_err(ServerError::Handler)?;

            // [periodic_data_identifier (1 byte), data_record (n bytes)]
            let did_low = (*did & 0xFF) as u8;
            let mut frame = Vec::new();
            // Unsolicited, so there is no request to answer with NRC 0x14.
            if frame.push(did_low).is_err() || frame.extend_from_slice(&data_buf[..len]).is_err() {
                return Err(ServerError::ResponseTooLong);
            }

            self.enqueue(client.clone(), frame)?;
            self.periodic.advance(*did, client, now);
        }

        Ok(())
    }

    // endregion: Periodic dispatch
}

// endregion: UdsServer
