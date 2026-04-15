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

use crate::gateway::{TCP_MAX_FRAME, TCP_MAX_OUTBOX};
use ace_client::{client::UdsClient, config::ClientConfig, event::ClientEvent, ClientError};
use ace_core::{FrameRead, FrameWrite};
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
    doip::constants::{
        DOIP_COMMON_EID_LEN, DOIP_COMMON_VIN_LEN, DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN,
    },
    DoipFrame,
};
use ace_sim::{
    clock::{Duration, Instant},
    io::NodeAddress,
    tcp_bus::TcpEvent,
};
use heapless::Vec;

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
}

// endregion: DoipConnectionConfig

// region: DoipConnectionPhase

#[derive(Debug, Clone, PartialEq, Eq)]
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
struct TargetState {
    /// ECU logical address.
    address: u16,

    /// UDS client for this target - owns P2 timer and event queue.
    client: UdsClient<1>,

    /// Current P2 timeout - updated from DSC response.
    p2: Duration,

    /// Current P2* timeout - updated from DSC response.
    p2_star: Duration,
}

impl TargetState {
    fn new(address: u16, default_p2: Duration, default_p2_star: Duration) -> Self {
        let config = ClientConfig::new(0, address)
            .with_p2_timeout(default_p2)
            .with_p2_extended_timeout(default_p2_star);

        Self {
            address,
            client: UdsClient::new(config, NodeAddress(address as u32)),
            p2: default_p2,
            p2_star: default_p2_star,
        }
    }

    /// Updates P2/P2* from a DiagnosticSessionControlResponse payload.
    ///
    /// DSC response payload after SID: [session_type, p2_high, p2_low, p2ext_high, p2ext_low] p2
    /// is in ms, p2ext is in units of 10ms.
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

        self.p2 = Duration::from_millis(p2_ms as u64);
        self.p2_star = Duration::from_millis(p2ext_10ms as u64);
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
pub struct DoipConnection<const MAX_TARGETS: usize = 8> {
    config: DoipConnectionConfig,
    phase: DoipConnectionPhase,

    /// Logical address of this tester - set by parent DoipTester.
    tester_address: u16,

    /// Per-ECU target state.
    targets: heapless::Vec<TargetState, MAX_TARGETS>,
    outbox: heapless::Vec<(NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    events: heapless::Vec<(ConnectionId, TargetId, DoipTesterEvent), 64>,
}

impl<const MAX_TARGETS: usize> DoipConnection<MAX_TARGETS> {
    fn new(tester_address: u16, config: DoipConnectionConfig) -> Self {
        Self {
            config,
            phase: DoipConnectionPhase::Disconnected,
            tester_address,
            targets: heapless::Vec::new(),
            outbox: heapless::Vec::new(),
            events: heapless::Vec::new(),
        }
    }

    pub fn id(&self) -> ConnectionId {
        ConnectionId(self.config.gateway_address)
    }

    // region: Target Management

    /// Ensures a UdsClient exists for the given target address. No-op if already registered.
    pub fn ensure_target(&mut self, target_address: u16) {
        if self.targets.iter().any(|t| t.address == target_address) {
            return;
        }

        let _ = self.targets.push(TargetState::new(
            target_address,
            self.config.default_p2,
            self.config.default_p2_star,
        ));
    }

    fn find_target_mut(&mut self, address: u16) -> Option<&mut TargetState> {
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

    fn connection_id(&self) -> ConnectionId {
        self.id()
    }

    pub fn on_connected(&mut self, _now: Instant) -> Result<(), DoipTesterError> {
        self.phase = DoipConnectionPhase::ActivationPending;
        self.send_routing_activation()
    }

    pub fn on_reset(&mut self) {
        self.phase = DoipConnectionPhase::Failed;

        let _ = self.events.push((
            self.connection_id(),
            TargetId(0),
            DoipTesterEvent::ConnectionReset,
        ));
    }

    pub fn on_refused(&mut self) {
        self.phase = DoipConnectionPhase::Failed;

        let _ = self.events.push((
            self.connection_id(),
            TargetId(0),
            DoipTesterEvent::ConnectionRefused,
        ));
    }

    pub fn on_timeout(&mut self) {
        self.phase = DoipConnectionPhase::Failed;

        let _ = self.events.push((
            self.connection_id(),
            TargetId(0),
            DoipTesterEvent::ConnectionTimeout,
        ));
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

        self.ensure_target(target_address);

        let target = self
            .find_target_mut(target_address)
            .ok_or(DoipTesterError::NotReady)?;

        target
            .client
            .request(uds_data, now)
            .map_err(DoipTesterError::Client)?;

        self.send_diagnostic_message(target_address, uds_data)
    }

    pub fn subscribe_periodic(&mut self, target_address: u16, did_low_byte: u8) {
        self.ensure_target(target_address);

        if let Some(t) = self.find_target_mut(target_address) {
            t.client.subscribe_periodic(did_low_byte);
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

    pub fn tick(&mut self, now: Instant) {
        let connection_id = self.connection_id();

        for target in self.targets.iter_mut() {
            let _ = target.client.tick(now);
            let uds_events: heapless::Vec<ClientEvent, 32> = target.client.drain_events().collect();

            for e in uds_events {
                if let ClientEvent::PositiveResponse {
                    sid: 0x10,
                    ref data,
                } = e
                {
                    target.update_timing_from_dsc(data);
                }

                let _ = self.events.push((
                    connection_id,
                    TargetId(target.address),
                    DoipTesterEvent::Uds(e),
                ));
            }
        }
    }

    // endregion: Tick

    // region: Outbox/Events

    pub fn drain_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> usize {
        let n = self.outbox.len();

        for item in self.outbox.drain(..) {
            let _ = out.push(item);
        }

        n
    }

    pub fn drain_events(
        &mut self,
    ) -> impl Iterator<Item = (ConnectionId, TargetId, DoipTesterEvent)> + '_ {
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

                let _ = self.events.push((
                    self.connection_id(),
                    TargetId(0),
                    DoipTesterEvent::ActivationSucceeded,
                ));
            }
            _ => {
                let code = resp.activation_code.into();
                self.phase = DoipConnectionPhase::Failed;

                let _ = self.events.push((
                    self.connection_id(),
                    TargetId(0),
                    DoipTesterEvent::ActivationDenied { code },
                ));
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

        self.ensure_target(source);

        if let Some(target) = self.find_target_mut(source) {
            let _ = target
                .client
                .handle(&NodeAddress(source as u32), uds_data, now);
        }

        Ok(())
    }

    fn on_diagnostic_nack(&mut self, payload_data: &[u8]) -> Result<(), DoipTesterError> {
        let source = u16::from_be_bytes([
            payload_data.get(0).copied().unwrap_or(0),
            payload_data.get(1).copied().unwrap_or(0),
        ]);
        let mut buf: heapless::Vec<u8, 256> = heapless::Vec::new();

        let _ = buf.extend_from_slice(&payload_data[..payload_data.len().min(256)]);

        let _ = self.events.push((
            self.connection_id(),
            TargetId(source),
            DoipTesterEvent::Uds(ClientEvent::Unsolicited { data: buf }),
        ));

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
        let mut frame: heapless::Vec<u8, TCP_MAX_FRAME> = heapless::Vec::new();

        {
            let mut slice = frame.as_mut();
            header
                .encode(&mut slice)
                .map_err(|_| DoipTesterError::Codec)?;
        }

        let _ = frame.extend_from_slice(&self.tester_address.to_be_bytes());
        let _ = frame.extend_from_slice(&target_address.to_be_bytes());
        let _ = frame.extend_from_slice(uds_data);

        self.outbox
            .push((gateway, frame))
            .map_err(|_| DoipTesterError::OutboxFull)
    }

    fn encode_and_send<T: FrameWrite<Error = DoipError>>(
        &mut self,
        payload_type: PayloadType,
        payload: &T,
    ) -> Result<(), DoipTesterError> {
        let mut payload_buf: heapless::Vec<u8, TCP_MAX_FRAME> = heapless::Vec::new();

        {
            let mut slice = payload_buf.as_mut();
            payload
                .encode(&mut slice)
                .map_err(|_| DoipTesterError::Codec)?;
        }

        let header = self.make_header(payload_type, payload_buf.len() as u32);

        let gateway = NodeAddress(self.config.gateway_address as u32);
        let mut frame = Vec::new();

        {
            let mut slice = frame.as_mut();
            header
                .encode(&mut slice)
                .map_err(|_| DoipTesterError::Codec)?;
            payload
                .encode(&mut slice)
                .map_err(|_| DoipTesterError::Codec)?;
        }

        self.outbox
            .push((gateway, frame))
            .map_err(|_| DoipTesterError::OutboxFull)
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
/// `MAX_CONNECTIONS` - max simultaneous TCP connections (default 8)
/// `MAX_TARGETS` - max ECU targets per connection (default 16)
pub struct DoipTester<const MAX_CONNECTIONS: usize = 8, const MAX_TARGETS: usize = 16> {
    /// Logical address of this tester device - shared across all connections.
    tester_address: u16,

    /// NodeAddress of this tester on the simulation TCP bus.
    address: NodeAddress,
    connections: heapless::Vec<DoipConnection<MAX_TARGETS>, MAX_CONNECTIONS>,

    /// Per-gateway metadata profiles accumulated from announcements.
    profiles: heapless::Vec<(u16, DoipNodeProfile), MAX_CONNECTIONS>,
}

impl<const MAX_CONNECTIONS: usize, const MAX_TARGETS: usize>
    DoipTester<MAX_CONNECTIONS, MAX_TARGETS>
{
    pub fn new(tester_address: u16, address: NodeAddress) -> Self {
        Self {
            tester_address,
            address,
            connections: heapless::Vec::new(),
            profiles: heapless::Vec::new(),
        }
    }

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }

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

        if self.connections.is_full() {
            return Err(DoipTesterError::TooManyConnections);
        }

        let _ = self
            .connections
            .push(DoipConnection::new(self.tester_address, config));

        Ok(id)
    }

    fn find_conn_mut(&mut self, id: ConnectionId) -> Option<&mut DoipConnection<MAX_TARGETS>> {
        self.connections.iter_mut().find(|c| c.id() == id)
    }

    fn find_conn_by_gateway_mut(
        &mut self,
        gateway_address: u16,
    ) -> Option<&mut DoipConnection<MAX_TARGETS>> {
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
    ) {
        if let Some(conn) = self.find_conn_mut(connection_id) {
            conn.subscribe_periodic(target_address, did_low_byte);
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

    pub fn tick(&mut self, now: Instant) {
        for conn in self.connections.iter_mut() {
            conn.tick(now);
        }
    }

    pub fn drain_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> usize {
        let mut total = 0;

        for conn in self.connections.iter_mut() {
            total += conn.drain_outbox(out);
        }

        total
    }

    // endregion: SimNode Surface

    // region: TcpEventHandler

    pub fn on_tcp_event(&mut self, event: &TcpEvent, now: Instant) {
        match event {
            TcpEvent::ConnectionEstablished { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    let _ = conn.on_connected(now);
                }
            }
            TcpEvent::ConnectionReset { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_reset();
                }
            }
            TcpEvent::ConnectionRefused { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_refused();
                }
            }
            TcpEvent::ConnectionTimeout { to, .. } => {
                let gateway = to.0 as u16;
                if let Some(conn) = self.find_conn_by_gateway_mut(gateway) {
                    conn.on_timeout();
                }
            }
            TcpEvent::ConnectionClosed { .. } => {}
        }
    }

    // endregion: TcpEventHandler

    // region: Event API

    pub fn drain_events(
        &mut self,
    ) -> impl Iterator<Item = (ConnectionId, TargetId, DoipTesterEvent)> + '_ {
        let mut all: heapless::Vec<(ConnectionId, TargetId, DoipTesterEvent), 128> = Vec::new();

        for conn in self.connections.iter_mut() {
            for ev in conn.drain_events() {
                let _ = all.push(ev);
            }
        }

        all.into_iter()
    }

    pub fn connection_phase(&self, id: ConnectionId) -> Option<&DoipConnectionPhase> {
        self.connections
            .iter()
            .find(|c| c.id() == id)
            .map(|c| c.phase())
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
pub enum DoipTesterEvent {
    ActivationSucceeded,
    ActivationDenied { code: u8 },
    ConnectionReset,
    ConnectionRefused,
    ConnectionTimeout,
    Uds(ClientEvent),
}

// endregion: DoipTesterEvent

// region: DoipTesterError

#[derive(Debug)]
pub enum DoipTesterError {
    NotReady,
    Codec,
    OutboxFull,
    TooManyConnections,
    DuplicateConnection,
    UnknownConnection,
    Client(ClientError),
}

// endregion: DoipTesterError
