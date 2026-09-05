//! DoipTester - models a physical DoIP diagnostic tester device.
//!
//! A real DoIP tester holds multiple simultaneous TCP connections, one per gateway it communicates
//! with. Each TCP connection (DoipConnection) can address multiple ECUs simultaneously -
//! DiagnosticMessage carries source_address and target_address so the tester can interleave
//! exchanges to different ECUs on a single connection.
//!
//! Example: TesterPresent (suppressed) to functional address and TransferData to ECU A
//! simultaneously on one TCP connection - two separate UdsClient instances, one per target, both
//! active on the same DoipConnection.
//!
//! P2/P2* timeouts are NOT configured upfront - they are learned dynamically from
//! DiagnosticSessionControlResponse per target. Defaults are used until a DSC response arrives.
//!
//! DiagnosticMessageAck is filtered at the DoIP layer and never surfaces as a UDS event. Responses
//! are de-multiplexed by source address on inbound DiagnosticMessage frames.

// region: Imports

// use crate::gateway::{TCP_MAX_FRAME, TCP_MAX_OUTBOX};
use ace_client::{ClientError, client::UdsClient, config::ClientConfig, event::ClientEvent};
use ace_core::storage::CapacityError;
use ace_core::{FrameRead, FrameWrite, Vec};
use ace_core::{
    clock::{Duration, Instant},
    io::{InboundFrame, LinkEvent, NodeAddress},
};
use ace_doip::{
    error::DoipError,
    ext::DoipFrameExt,
    header::{DoipHeader, PayloadType, ProtocolVersion},
    payload::{
        ActivationCode, ActivationType, AliveCheckResponse, EntityStatusResponse,
        RoutingActivationRequest, RoutingActivationResponse, VehicleAnnouncementMessage,
    },
};
use ace_proto::{
    DoipFrame,
    doip::constants::{
        DOIP_COMMON_EID_LEN, DOIP_COMMON_VIN_LEN, DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN,
    },
};

// endregion: Imports

// region: ConnectionId

/// Identifies a specific TCP connection within a `DoipTester`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionId(pub u16);

// region: ConnectionId

// region: TargetId

/// Identifies a specific ECU target within a `DoipConnection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetId(pub u16);

// endregion: TargetId

// region: DoipNodeProfile

/// Accumulated metadata about a DoIP gateway node.
///
/// Updated from `VehicleAnnouncementMessage` and `EntityStatusResponse` frames as they arrive.
/// Provides a full metadata profile of the gateway without requiring explicit requests in normal
/// operation - vehicle announcements arrive automatically on network join.
#[derive(Debug, Clone, Default)]
pub struct DoipNodeProfile {
    pub vin: Option<[u8; DOIP_COMMON_VIN_LEN]>,
    pub logical_address: Option<u16>,
    pub eid: Option<[u8; DOIP_COMMON_EID_LEN]>,
    pub gid: Option<[u8; DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN]>,
    pub further_action: Option<u8>,
    pub max_open_sockets: Option<u8>,
    pub currently_open_sockets: Option<u8>,
    pub max_data_size: Option<u32>,
}

impl DoipNodeProfile {
    pub fn update_from_announcement(&mut self, msg: &VehicleAnnouncementMessage) {
        self.vin = Some(msg.vin);
        self.logical_address = Some(u16::from_be_bytes(msg.logical_address));
        self.eid = Some(msg.eid);
        self.gid = Some(msg.gid);
        self.further_action = Some((&msg.further_action).into());
    }

    pub fn update_from_entity_status(&mut self, resp: &EntityStatusResponse) {
        self.max_open_sockets = Some(u8::from_be_bytes(resp.max_concurrent_sockets));
        self.currently_open_sockets = Some(u8::from_be_bytes(resp.currently_open_sockets));
        self.max_data_size = Some(u32::from_be_bytes(resp.max_data_size));
    }
}

// endregion: DoipNodeProfile

// region: DoipConnectionConfig

/// Configuration for a single TCP connection attempt.
///
/// Does not include P2/P2* - those are learned from the server.
#[derive(Debug, Clone)]
pub struct DoipConnectionConfig {
    /// Logical address  of the gateway to connect to.
    pub gateway_address: u16,

    /// Activation type to use for routing activation.
    pub activation_type: ActivationType,

    /// OEM-specific data for activation.
    pub oem_data: [u8; 4],

    /// Default P2 timeout before a DSC response is received.
    pub default_p2: Duration,

    /// Default P2* timeout before a DSC response is received.
    pub default_p2_star: Duration,

    /// Margin added to server-advertised P2/P2* timing once learned from a
    /// `DiagnosticSessionControlResponse`, before it is applied to the live
    /// client. Per ISO 14229-1 the client timeout should match or slightly
    /// exceed the server's, so this should not be zero in production;
    /// `Duration::from_millis(0)` applies the learned values verbatim.
    pub timing_tolerance: Duration,

    /// DoIP Protocol version to use for all outbound frames.
    pub protocol_version: ProtocolVersion,
}

impl DoipConnectionConfig {
    pub fn new(gateway_address: u16) -> Self {
        Self {
            gateway_address,
            activation_type: ActivationType::Default,
            oem_data: [0u8; 4],
            default_p2: Duration::from_millis(500),
            default_p2_star: Duration::from_millis(5_000),
            timing_tolerance: Duration::from_millis(50),
            protocol_version: ProtocolVersion::Iso13400_2012,
        }
    }

    pub fn with_activation_type(mut self, t: ActivationType) -> Self {
        self.activation_type = t;
        self
    }

    pub fn with_oem_data(mut self, data: [u8; 4]) -> Self {
        self.oem_data = data;
        self
    }

    pub fn with_timing_tolerance(mut self, tolerance: Duration) -> Self {
        self.timing_tolerance = tolerance;
        self
    }
}

// endregion: DoipConnectionConfig

// region: DoipConnectionPhase

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum DoipConnectionPhase {
    Disconnected,
    ActivationPending,
    Active,
    Failed,
}

// endregion: DoipConnectionPhase

// region: TargetState

/// Per-ECU state within a single DoipConnection
///
/// Each target has its own UdsClient tracking P2/P2* independently. P2/P2* are updated when a
/// DiagnosticSessionControlResponse arrives from this target.
#[derive(Debug)]
pub struct TargetState<
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
> {
    /// ECU logical address.
    pub address: u16,

    /// UDS client for this target - owns P2 timer and event queue.
    pub client: UdsClient<
        PENDING,
        SIM_MAX_FRAME,
        SIM_MAX_OUTBOX,
        MAX_TARGET_EVENTS,
        PERIODIC_DIDS,
        MAX_DATA,
    >,

    /// Current P2 timeout - updated from DSC response.
    p2: Duration,

    /// Current P2* timeout - updated from DSC response.
    p2_star: Duration,

    /// Margin applied to server-advertised timing before it takes effect on
    /// `client`. See [`DoipConnectionConfig::timing_tolerance`].
    timing_tolerance: Duration,
}

impl<
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
> TargetState<PENDING, SIM_MAX_FRAME, SIM_MAX_OUTBOX, MAX_EVENTS, PERIODIC_DIDS, MAX_DATA>
{
    fn new(
        address: u16,
        default_p2: Duration,
        default_p2_star: Duration,
        timing_tolerance: Duration,
    ) -> Self {
        let config = ClientConfig::new(0, address)
            .with_p2_timeout(default_p2)
            .with_p2_extended_timeout(default_p2_star);

        Self {
            address,
            client: UdsClient::new(config, NodeAddress(address as u32)),
            p2: default_p2,
            p2_star: default_p2_star,
            timing_tolerance,
        }
    }

    /// Updates P2/P2* from a DiagnosticSessionControlResponse payload, and
    /// applies the learned values to the live [`UdsClient`] used for every
    /// subsequent request to this target.
    ///
    /// DSC response payload after SID: [session_type, p2_high, p2_low, p2ext_high, p2ext_low].
    /// `p2` is in whole milliseconds; `p2ext` is in units of 10ms per
    /// ISO 14229-1, so it must be multiplied by 10 to get milliseconds.
    fn update_timing_from_dsc(&mut self, uds_payload: &[u8]) {
        if uds_payload.len() < 5 {
            return;
        }

        let p2_ms = u16::from_be_bytes([
            uds_payload.get(1).copied().unwrap_or(0),
            uds_payload.get(2).copied().unwrap_or(0),
        ]);

        let p2ext_10ms = u16::from_be_bytes([
            uds_payload.get(3).copied().unwrap_or(0),
            uds_payload.get(4).copied().unwrap_or(0),
        ]);

        self.p2 = Duration::from_millis(u64::from(p2_ms));
        self.p2_star = Duration::from_millis(u64::from(p2ext_10ms) * 10);

        self.client
            .set_timing(self.p2, self.p2_star, self.timing_tolerance);
    }
}

// endregion: TargetState

// region: DoipConnection

/// A single TCP connection within a DoipTester.
///
/// Manages routing activation and multiplexes UDS exchanges to multiple ECU targets simultaneously
/// on one TCP connection.
///
/// `MAX_TARGETS` - max concurrent ECU targets on this connection.
#[derive(Debug)]
pub struct DoipConnection<
    // region: DoipConnection Constants
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    // endregion: DoipConnection Constants

    // region: Target Constants
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
    // endregion: Target Constants
> {
    config: DoipConnectionConfig,
    phase: DoipConnectionPhase,

    /// Logical address of this tester - set by parent DoipTester.
    tester_address: u16,

    /// Per-ECU target state.
    pub targets: Vec<
        TargetState<
            PENDING,
            SIM_MAX_FRAME,
            SIM_MAX_OUTBOX,
            MAX_TARGET_EVENTS,
            PERIODIC_DIDS,
            MAX_DATA,
        >,
        MAX_TARGETS,
    >,
    outbox: Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    events: Vec<(ConnectionId, TargetId, DoipTesterEvent<MAX_DATA>), MAX_CONNECTION_ACTIONS>,
}

impl<
    // region: DoipConnection Constants
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    // endregion: DoipConnection Constants

    // region: Target Constants
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
    // endregion: Target Constants
>
    DoipConnection<
        MAX_TARGETS,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        MAX_CONNECTION_ACTIONS,
        PENDING,
        SIM_MAX_FRAME,
        SIM_MAX_OUTBOX,
        MAX_TARGET_EVENTS,
        PERIODIC_DIDS,
        MAX_DATA,
    >
{
    pub fn new(tester_address: u16, config: DoipConnectionConfig) -> Self {
        Self {
            config,
            phase: DoipConnectionPhase::Disconnected,
            tester_address,
            targets: Vec::new(),
            outbox: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn id(&self) -> ConnectionId {
        ConnectionId(self.config.gateway_address)
    }

    // region: Target Management

    /// Ensures a UdsClient exists for the given target address. No-op if already registered.
    pub fn ensure_target(&mut self, target_address: u16) -> Result<(), DoipTesterError> {
        if self.targets.iter().any(|t| t.address == target_address) {
            return Ok(());
        }

        self.targets
            .push(TargetState::new(
                target_address,
                self.config.default_p2,
                self.config.default_p2_star,
                self.config.timing_tolerance,
            ))
            .map_err(|_| DoipTesterError::TooManyTargets)
    }

    fn find_target_mut(
        &mut self,
        address: u16,
    ) -> Option<
        &mut TargetState<
            PENDING,
            SIM_MAX_FRAME,
            SIM_MAX_OUTBOX,
            MAX_TARGET_EVENTS,
            PERIODIC_DIDS,
            MAX_DATA,
        >,
    > {
        self.targets.iter_mut().find(|t| t.address == address)
    }

    // endregion: Target Management

    // region: Lifecycle

    pub fn phase(&self) -> &DoipConnectionPhase {
        &self.phase
    }

    pub fn is_active(&self) -> bool {
        self.phase == DoipConnectionPhase::Active
    }

    pub fn target_pending_count(&self, target_address: u16) -> Option<usize> {
        self.targets
            .iter()
            .find(|t| t.address == target_address)
            .map(|t| t.client.pending_count())
    }

    fn connection_id(&self) -> ConnectionId {
        self.id()
    }

    pub fn on_connected(&mut self, _now: Instant) -> Result<(), DoipTesterError> {
        self.phase = DoipConnectionPhase::ActivationPending;
        self.send_routing_activation()
    }

    pub fn on_reset(&mut self) -> Result<(), DoipTesterError> {
        self.phase = DoipConnectionPhase::Failed;
        self.emit(TargetId(0), DoipTesterEvent::ConnectionReset)
    }

    pub fn on_refused(&mut self) -> Result<(), DoipTesterError> {
        self.phase = DoipConnectionPhase::Failed;
        self.emit(TargetId(0), DoipTesterEvent::ConnectionRefused)
    }

    pub fn on_timeout(&mut self) -> Result<(), DoipTesterError> {
        self.phase = DoipConnectionPhase::Failed;
        self.emit(TargetId(0), DoipTesterEvent::ConnectionTimeout)
    }

    /// Records an observed event for the caller to drain.
    ///
    /// Fails rather than dropping. The event queue is how a caller learns what the
    /// connection saw, so a discarded event leaves them unable to distinguish a reset
    /// link from a quiet one.
    fn emit(
        &mut self,
        target: TargetId,
        event: DoipTesterEvent<MAX_DATA>,
    ) -> Result<(), DoipTesterError> {
        self.events
            .push((self.connection_id(), target, event))
            .map_err(|_| DoipTesterError::EventBufferOverflow)
    }

    // endregion: Lifecycle

    // region: Request API

    /// Sends a UDS request to the given target ECU.
    pub fn request(
        &mut self,
        target_address: u16,
        uds_data: &[u8],
        now: Instant,
    ) -> Result<(), DoipTesterError> {
        if !self.is_active() {
            return Err(DoipTesterError::NotReady);
        }

        self.ensure_target(target_address)?;

        let target = self
            .find_target_mut(target_address)
            .ok_or(DoipTesterError::NotReady)?;

        target
            .client
            .request(uds_data, now)
            .map_err(DoipTesterError::Client)?;

        self.send_diagnostic_message(target_address, uds_data)
    }

    pub fn subscribe_periodic(
        &mut self,
        target_address: u16,
        did_low_byte: u8,
    ) -> Result<(), DoipTesterError> {
        self.ensure_target(target_address)?;

        match self.find_target_mut(target_address) {
            Some(t) => t
                .client
                .subscribe_periodic(did_low_byte)
                .map_err(DoipTesterError::Client),
            None => Err(DoipTesterError::TooManyTargets),
        }
    }

    // endregion: Request API

    // region: Inbound Frame Handling

    pub fn handle(&mut self, data: &[u8], now: Instant) -> Result<(), DoipTesterError> {
        let frame = DoipFrame::from_slice(data);

        if frame.validate_header().is_err() {
            return Ok(());
        }

        let payload_type = match frame.payload_type() {
            Some(Ok(pt)) => pt,
            _ => return Ok(()),
        };

        let payload_data = frame.payload_bytes().unwrap_or(&[]);

        match payload_type {
            PayloadType::RoutingActivationResponse => self.on_activation_response(payload_data),
            PayloadType::DiagnosticMessage => self.on_diagnostic_message(payload_data, now),
            PayloadType::DiagnosticMessageAck => Ok(()),
            PayloadType::DiagnosticMessageNack => self.on_diagnostic_nack(payload_data),
            PayloadType::AliveCheckRequest => self.on_alive_check_request(),
            _ => Ok(()),
        }
    }

    // endregion: Inbound Frame Handling

    // region: Tick

    /// Returns `Err` if a bounded buffer could not absorb what was drained into
    /// it. Fallible for the same reason `UdsServer::tick` and `UdsClient::tick`
    /// are: a tick that cannot report failure can only drop it.
    pub fn tick(&mut self, now: Instant) -> Result<(), DoipTesterError> {
        let connection_id = self.connection_id();

        for target in self.targets.iter_mut() {
            let _ = target.client.tick(now);
            let uds_events: Vec<ClientEvent<MAX_DATA>, MAX_TARGET_EVENTS> =
                Vec::from_iter_checked(target.client.drain_events())
                    .map_err(|_| DoipTesterError::EventBufferOverflow)?;

            // Note: a DiagnosticSessionControl positive response (SID 0x10)
            // is never observed here. It arrives via on_diagnostic_message
            // on the frame-handling path, which drains target.client's
            // event queue on arrival - see the DSC timing comment there.
            // tick() only advances P2/P2* timers and drains timer-driven
            // events (e.g. Timeout), so a SID-0x10 check here would be dead
            // code by construction.
            for e in uds_events {
                // Pushed through the field rather than `emit`: `self.targets` is
                // borrowed mutably by the loop, so a `&mut self` method would clash.
                self.events
                    .push((
                        connection_id,
                        TargetId(target.address),
                        DoipTesterEvent::Uds(e),
                    ))
                    .map_err(|_| DoipTesterError::EventBufferOverflow)?;
            }
        }

        Ok(())
    }

    // endregion: Tick

    // region: Outbox/Events

    pub fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
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

    pub fn drain_events(
        &mut self,
    ) -> impl Iterator<Item = (ConnectionId, TargetId, DoipTesterEvent<MAX_DATA>)> + '_ {
        self.events.drain(..)
    }

    // endregion: Outbox/Events

    // region: Frame Handlers

    fn on_activation_response(&mut self, payload_data: &[u8]) -> Result<(), DoipTesterError> {
        if self.phase != DoipConnectionPhase::ActivationPending {
            return Ok(());
        }

        let mut cursor = payload_data;
        let resp =
            RoutingActivationResponse::decode(&mut cursor).map_err(|_| DoipTesterError::Codec)?;

        match resp.activation_code {
            ActivationCode::SuccessfullyActivated => {
                self.phase = DoipConnectionPhase::Active;

                self.emit(TargetId(0), DoipTesterEvent::ActivationSucceeded)?;
            }
            _ => {
                let code = resp.activation_code.into();
                self.phase = DoipConnectionPhase::Failed;

                self.emit(TargetId(0), DoipTesterEvent::ActivationDenied { code })?;
            }
        }

        Ok(())
    }

    fn on_diagnostic_message(
        &mut self,
        payload_data: &[u8],
        now: Instant,
    ) -> Result<(), DoipTesterError> {
        let source = u16::from_be_bytes([
            payload_data.get(0).copied().unwrap_or(0),
            payload_data.get(1).copied().unwrap_or(0),
        ]);
        let uds_data = payload_data.get(4..).unwrap_or(&[]);

        if uds_data.is_empty() {
            return Ok(());
        }

        self.ensure_target(source)?;

        let conn_id = self.connection_id();
        let target_id = TargetId(u16::from(source));
        let tester_address = self.tester_address;

        if let Some(target) = self.find_target_mut(source) {
            // Addressed to us: a response arrives at the tester's own address.
            let frame = InboundFrame::new(
                NodeAddress(source as u32),
                NodeAddress(u32::from(tester_address)),
                uds_data,
            );
            let _ = target.client.handle(&frame, now);

            let client_events: Vec<ClientEvent<MAX_DATA>, MAX_TARGET_EVENTS> =
                Vec::from_iter_checked(target.client.drain_events())
                    .map_err(|_| DoipTesterError::EventBufferOverflow)?;

            // The DSC response is delivered here, on the frame-arrival path
            // - never through tick(), which only re-drains target.client's
            // queue on a timer and finds nothing left once handle() above
            // has already consumed it. This is the only place a
            // PositiveResponse{sid: 0x10} event is ever actually observed,
            // so this is where learned P2/P2* timing must be applied.
            for client_event in &client_events {
                if let ClientEvent::PositiveResponse { sid: 0x10, data } = client_event {
                    target.update_timing_from_dsc(data);
                }
            }

            for client_event in client_events {
                let _ = self
                    .events
                    .push((conn_id, target_id, DoipTesterEvent::Uds(client_event)));
            }
        }

        Ok(())
    }

    fn on_diagnostic_nack(&mut self, payload_data: &[u8]) -> Result<(), DoipTesterError> {
        let source = u16::from_be_bytes([
            payload_data.get(0).copied().unwrap_or(0),
            payload_data.get(1).copied().unwrap_or(0),
        ]);
        let mut buf: Vec<u8, MAX_DATA> = Vec::new();

        buf.extend_from_slice(payload_data)
            .map_err(|_| DoipTesterError::FrameTooLong)?;

        self.emit(
            TargetId(source),
            DoipTesterEvent::Uds(ClientEvent::Unsolicited { data: buf }),
        )?;

        Ok(())
    }

    fn on_alive_check_request(&mut self) -> Result<(), DoipTesterError> {
        let resp = AliveCheckResponse {
            source_address: self.tester_address.to_be_bytes(),
        };
        self.encode_and_send(PayloadType::AliveCheckResponse, &resp)
    }

    // endregion: Frame Handlers

    // region: Frame Construction Helpers

    fn protocol_version(&self) -> ProtocolVersion {
        self.config.protocol_version
    }

    fn inverse_version(&self) -> u8 {
        !(self.protocol_version() as u8)
    }

    fn make_header(&self, payload_type: PayloadType, payload_length: u32) -> DoipHeader {
        DoipHeader {
            protocol_version: self.protocol_version(),
            inverse_protocol_version: self.inverse_version(),
            payload_type,
            payload_length,
        }
    }

    fn send_routing_activation(&mut self) -> Result<(), DoipTesterError> {
        let req = RoutingActivationRequest {
            source_address: self.tester_address.to_be_bytes(),
            activation_type: self.config.activation_type.clone(),
            reserved: [0u8; 4],
            reserved_for_oem: self.config.oem_data,
        };

        self.encode_and_send(PayloadType::RoutingActivationRequest, &req)
    }

    fn send_diagnostic_message(
        &mut self,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<(), DoipTesterError> {
        let payload_len = 4 + uds_data.len();
        let header = self.make_header(PayloadType::DiagnosticMessage, payload_len as u32);
        let gateway = NodeAddress(self.config.gateway_address as u32);

        let mut header_staging = [0u8; 8];
        let mut header_slice: &mut [u8] = &mut header_staging;

        header
            .encode(&mut header_slice)
            .map_err(|_| DoipTesterError::Codec)?;

        // Calculate AFTER encoding
        let header_len = 8 - header_slice.len();

        let mut frame: Vec<u8, TCP_MAX_FRAME> = Vec::new();

        frame
            .extend_from_slice(&header_staging[..header_len])
            .map_err(|_| DoipTesterError::Codec)?;

        frame
            .extend_from_slice(&self.tester_address.to_be_bytes())
            .map_err(|_| DoipTesterError::Codec)?;

        frame
            .extend_from_slice(&target_address.to_be_bytes())
            .map_err(|_| DoipTesterError::Codec)?;

        frame
            .extend_from_slice(uds_data)
            .map_err(|_| DoipTesterError::Codec)?;

        self.outbox
            .push((gateway, frame))
            .map_err(|_| DoipTesterError::OutboxFull)?;

        Ok(())
    }

    fn encode_and_send<T: FrameWrite<Error = DoipError>>(
        &mut self,
        payload_type: PayloadType,
        payload: &T,
    ) -> Result<(), DoipTesterError> {
        let mut payload_staging = [0u8; TCP_MAX_FRAME];
        let mut payload_slice: &mut [u8] = &mut payload_staging;

        payload
            .encode(&mut payload_slice)
            .map_err(|_| DoipTesterError::Codec)?;

        let payload_len = TCP_MAX_FRAME - payload_slice.len();

        let header = self.make_header(payload_type, payload_len as u32);
        let gateway = NodeAddress(self.config.gateway_address as u32);

        let mut header_staging = [0u8; 8];
        let mut header_slice: &mut [u8] = &mut header_staging;

        header
            .encode(&mut header_slice)
            .map_err(|_| DoipTesterError::Codec)?;

        let header_len = 8 - header_slice.len();
        let mut frame: Vec<u8, TCP_MAX_FRAME> = Vec::new();

        frame
            .extend_from_slice(&header_staging[..header_len])
            .map_err(|_| DoipTesterError::Codec)?;

        frame
            .extend_from_slice(&payload_staging[..payload_len])
            .map_err(|_| DoipTesterError::Codec)?;

        self.outbox
            .push((gateway, frame))
            .map_err(|_| DoipTesterError::OutboxFull)?;

        Ok(())
    }

    // endregion: Frame Construction Helpers
}

// endregion: DoipConnection

// region: DoipTester

/// A DoIP diagnostic tester device.
///
/// Owns multiple simultaneous TCP connection (`DoipConnection`), each of which can address
/// multiple ECUs simultaneously.
///
/// # Memory footprint
///
/// All backing storage is inline (`heapless::Vec`), so the stack/static
/// footprint of `UdsClient` grows with its const generics as:
///
/// ```text
/// size(UdsClient) = O(PENDING)
///                  + O(SIM_MAX_OUTBOX * SIM_MAX_FRAME)
///                  + O(MAX_EVENTS * MAX_DATA)
///                  + O(PERIODIC_DIDS)
///                  + O(1)
/// ```
///
/// `SIM_MAX_OUTBOX * SIM_MAX_FRAME` and `MAX_EVENTS * MAX_DATA` are
/// **products** of two independent generics, not sums: each outbox slot
/// holds a full `SIM_MAX_FRAME`-byte frame, and each buffered event holds a
/// full `MAX_DATA`-byte record. Doubling either factor in one of these pairs
/// roughly doubles that field's size; doubling both roughly quadruples it.
/// `PENDING` and `PERIODIC_DIDS` scale linearly and are comparatively cheap.
///
/// For the exact byte count of a given configuration (padding/alignment
/// included), use [`UdsClient::FOOTPRINT_BYTES`] rather than computing the
/// formula above by hand.
pub struct DoipTester<
    // region: DoipTester Constants
    const MAX_CONNECTIONS: usize,
    // endregion: DoipTester Constants

    // region: DoipConnection Constants
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    // endregion: DoipConnection Constants

    // region: Target Constants
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
    // endregion: Target Constants
> {
    /// Logical address of this tester device - shared across all connections.
    tester_address: u16,

    /// NodeAddress of this tester on the simulation TCP bus.
    address: NodeAddress,
    pub connections: Vec<
        DoipConnection<
            MAX_TARGETS,
            TCP_MAX_FRAME,
            TCP_MAX_OUTBOX,
            MAX_CONNECTION_ACTIONS,
            PENDING,
            SIM_MAX_FRAME,
            SIM_MAX_OUTBOX,
            MAX_TARGET_EVENTS,
            PERIODIC_DIDS,
            MAX_DATA,
        >,
        MAX_CONNECTIONS,
    >,

    /// Per-gateway metadata profiles accumulated from announcements.
    profiles: Vec<(u16, DoipNodeProfile), MAX_CONNECTIONS>,
}

impl<
    const MAX_CONNECTIONS: usize,
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
>
    DoipTester<
        MAX_CONNECTIONS,
        MAX_TARGETS,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        MAX_CONNECTION_ACTIONS,
        PENDING,
        SIM_MAX_FRAME,
        SIM_MAX_OUTBOX,
        MAX_TARGET_EVENTS,
        PERIODIC_DIDS,
        MAX_DATA,
    >
{
    pub fn new(tester_address: u16, address: NodeAddress) -> Self {
        Self {
            tester_address,
            address,
            connections: Vec::new(),
            profiles: Vec::new(),
        }
    }

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }

    /// Exact stack footprint in bytes for this configuration.
    pub const FOOTPRINT_BYTES: usize = core::mem::size_of::<Self>();

    // region: Connection Management

    /// Opens a new connection with the given config. Returns the `ConnectionId` for subsequent
    /// operations. The caller must also call `TcpSimBus::connect()` separately.
    pub fn open_connection(
        &mut self,
        config: DoipConnectionConfig,
    ) -> Result<ConnectionId, DoipTesterError> {
        let id = ConnectionId(config.gateway_address);
        if self.connections.iter().any(|c| c.id() == id) {
            return Err(DoipTesterError::DuplicateConnection);
        }

        if self.connections.len() >= MAX_CONNECTIONS {
            return Err(DoipTesterError::TooManyConnections);
        }

        let _ = self
            .connections
            .push(DoipConnection::new(self.tester_address, config));

        Ok(id)
    }

    pub fn find_conn_mut(
        &mut self,
        id: ConnectionId,
    ) -> Option<
        &mut DoipConnection<
            MAX_TARGETS,
            TCP_MAX_FRAME,
            TCP_MAX_OUTBOX,
            MAX_CONNECTION_ACTIONS,
            PENDING,
            SIM_MAX_FRAME,
            SIM_MAX_OUTBOX,
            MAX_TARGET_EVENTS,
            PERIODIC_DIDS,
            MAX_DATA,
        >,
    > {
        self.connections.iter_mut().find(|c| c.id() == id)
    }

    fn find_conn_by_gateway_mut(
        &mut self,
        gateway_address: u16,
    ) -> Option<
        &mut DoipConnection<
            MAX_TARGETS,
            TCP_MAX_FRAME,
            TCP_MAX_OUTBOX,
            MAX_CONNECTION_ACTIONS,
            PENDING,
            SIM_MAX_FRAME,
            SIM_MAX_OUTBOX,
            MAX_TARGET_EVENTS,
            PERIODIC_DIDS,
            MAX_DATA,
        >,
    > {
        self.connections
            .iter_mut()
            .find(|c| c.config.gateway_address == gateway_address)
    }

    // endregion: Connection Management

    // region: Node Profile Management

    fn find_or_create_profile(&mut self, gateway_address: u16) -> &mut DoipNodeProfile {
        if let Some(pos) = self
            .profiles
            .iter()
            .position(|(a, _)| *a == gateway_address)
        {
            return &mut self.profiles[pos].1;
        }

        let _ = self
            .profiles
            .push((gateway_address, DoipNodeProfile::default()));
        let last = self.profiles.len() - 1;
        &mut self.profiles[last].1
    }

    pub fn profile(&self, gateway_address: u16) -> Option<&DoipNodeProfile> {
        self.profiles
            .iter()
            .find(|(a, _)| *a == gateway_address)
            .map(|(_, p)| p)
    }

    // endregion: Node Profile Management

    // region: Request API

    pub fn request(
        &mut self,
        connection_id: ConnectionId,
        target_address: u16,
        uds_data: &[u8],
        now: Instant,
    ) -> Result<(), DoipTesterError> {
        self.find_conn_mut(connection_id)
            .ok_or(DoipTesterError::UnknownConnection)?
            .request(target_address, uds_data, now)
    }

    pub fn subscribe_periodic(
        &mut self,
        connection_id: ConnectionId,
        target_address: u16,
        did_low_byte: u8,
    ) -> Result<(), DoipTesterError> {
        match self.find_conn_mut(connection_id) {
            Some(conn) => conn.subscribe_periodic(target_address, did_low_byte),
            None => Err(DoipTesterError::UnknownConnection),
        }
    }

    // endregion: Request API

    // region: SimNode Surface

    pub fn handle(
        &mut self,
        src: &NodeAddress,
        data: &[u8],
        now: Instant,
    ) -> Result<(), DoipTesterError> {
        let gateway_address = src.0 as u16;

        let frame = DoipFrame::from_slice(data);
        if frame.validate_header().is_ok() {
            if let Some(Ok(payload_type)) = frame.payload_type() {
                let payload_data = frame.payload_bytes().unwrap_or(&[]);
                self.handle_profile_frame(gateway_address, payload_type, payload_data);
            }
        }

        if let Some(conn) = self.find_conn_by_gateway_mut(gateway_address) {
            conn.handle(data, now)?;
        }

        Ok(())
    }

    /// Fallible so a per-connection tick failure is reported rather than dropped.
    pub fn tick(&mut self, now: Instant) -> Result<(), DoipTesterError> {
        for conn in self.connections.iter_mut() {
            conn.tick(now)?;
        }

        Ok(())
    }

    pub fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> Result<usize, CapacityError> {
        let mut total = 0;

        // Each connection's drain is all-or-nothing, but this aggregate is not: an
        // earlier connection may already have moved its frames before a later one
        // finds `out` full. The count returned on success is therefore exact, and on
        // failure the caller must treat `out` as holding an unknown prefix.
        for conn in self.connections.iter_mut() {
            total += conn.drain_outbox(out)?;
        }

        Ok(total)
    }

    // endregion: SimNode Surface

    // region: LinkEvent dispatch

    pub fn on_tcp_event(&mut self, event: &LinkEvent, now: Instant) -> Result<(), DoipTesterError> {
        match event {
            LinkEvent::ConnectionEstablished { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_connected(now)?;
                }
            }
            LinkEvent::ConnectionReset { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_reset()?;
                }
            }
            LinkEvent::ConnectionRefused { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_refused()?;
                }
            }
            LinkEvent::ConnectionTimeout { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_timeout()?;
                }
            }
            LinkEvent::ConnectionClosed { .. } => {}
        }

        Ok(())
    }

    // endregion: LinkEvent dispatch

    // region: Event API

    /// Drains every connection's observed events.
    ///
    /// Chains the per-connection queues lazily rather than copying into an
    /// intermediate buffer. The previous implementation aggregated into a `Vec`
    /// bounded by `MAX_CONNECTION_ACTIONS` - a constant describing a *single*
    /// connection's pending actions - so a tester with several busy connections
    /// silently discarded the overflow. There is no aggregate bound to get wrong now.
    pub fn drain_events(
        &mut self,
    ) -> impl Iterator<Item = (ConnectionId, TargetId, DoipTesterEvent<MAX_DATA>)> + '_ {
        self.connections
            .iter_mut()
            .flat_map(|conn| conn.drain_events())
    }

    pub fn connection_phase(&self, id: ConnectionId) -> Option<&DoipConnectionPhase> {
        self.connections
            .iter()
            .find(|c| c.id() == id)
            .map(|c| c.phase())
    }

    pub fn connection_pending_count(&self, id: ConnectionId, target_address: u16) -> usize {
        self.connections
            .iter()
            .find(|c| c.id() == id)
            .and_then(|conn| conn.target_pending_count(target_address))
            .unwrap_or(0)
    }

    // endregion: Event API

    // region: Profile Frame Handling

    fn handle_profile_frame(
        &mut self,
        gateway_address: u16,
        payload_type: PayloadType,
        payload_data: &[u8],
    ) {
        match payload_type {
            PayloadType::VehicleAnnouncementMessage => {
                let mut cursor = payload_data;
                if let Ok(msg) = VehicleAnnouncementMessage::decode(&mut cursor) {
                    self.find_or_create_profile(gateway_address)
                        .update_from_announcement(&msg);
                }
            }
            PayloadType::EntityStatusResponse => {
                let mut cursor = payload_data;
                if let Ok(msg) = EntityStatusResponse::decode(&mut cursor) {
                    self.find_or_create_profile(gateway_address)
                        .update_from_entity_status(&msg);
                }
            }
            _ => {}
        }
    }

    // endregion: Profile Frame Handling
}

// endregion: DoipTester

// region: DoipTesterEvent

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoipTesterEvent<const MAX_DATA: usize> {
    ActivationSucceeded,
    ActivationDenied { code: u8 },
    ConnectionReset,
    ConnectionRefused,
    ConnectionTimeout,
    Uds(ClientEvent<MAX_DATA>),
}

// endregion: DoipTesterEvent

// region: DoipTesterError

#[derive(Debug)]
pub enum DoipTesterError {
    NotReady,
    Codec,
    OutboxFull,
    TooManyConnections,

    /// The per-connection target table is full, so an ECU target could not be
    /// registered. Reported rather than dropped: an unregistered target silently
    /// discards every request addressed to it.
    TooManyTargets,
    DuplicateConnection,
    UnknownConnection,
    /// A bounded event buffer could not hold everything drained into it.
    EventBufferOverflow,

    /// Received data exceeded the bounded buffer it was being assembled into.
    FrameTooLong,
    Client(ClientError),
}

// endregion: DoipTesterError
