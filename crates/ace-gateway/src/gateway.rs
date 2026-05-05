//! DoIP gateway state machine.
//!
//! The gateway sits between two simulation buses:
//!     - TCP bus (DoIP side): receives DoIP frames from testers
//!     - CAN bus (ISO-TP side): sends/receives raw CAN-addressed UDS bytes
//!
//! It owns one ConnectionState per tester (up to MAX_TESTERS) and a PendingRouteTable that matches
//! CAN responses back to their tester.
//!
//! The gateway is a SimNode on the TCP bus. It communicates with the IsoTpNode on the CAN bus via
//! raw UDS bytes addressed by CAN ID.

// region: Imports

use ace_can::{
    IsoTpAddressingMode, ReassembleResult, Reassembler, ReassemblerConfig, SegmentResult,
    Segmenter, SegmenterConfig,
};
use ace_core::FrameWrite;
use ace_doip::{
    error::DoipError,
    ext::DoipFrameExt,
    header::{DoipHeader, PayloadType, ProtocolVersion},
    payload::{
        AliveCheckRequest, AliveCheckResponse, DiagnosticAckCode, DiagnosticMessageAck,
        DiagnosticMessageNack, DiagnosticNackCode, GenericNack, NackCode,
    },
    session::{ActivationAuthProvider, ActivationStateMachine, ConnectionEvent, ConnectionState},
};
use ace_proto::{doip::constants::DOIP_HEADER_LEN, DoipFrame};
use ace_sim::{clock::Instant, io::NodeAddress};

use crate::{
    config::GatewayConfig,
    router::{PendingRoute, PendingRouteTable},
};

// endregion: Imports

// region: Capacity Constants

/// Max DoIP frame size for the TCP bus side.
pub const TCP_MAX_FRAME: usize = 4096 + DOIP_HEADER_LEN + 4; // UDS + DoIP header + addresses

/// Max outbox depth on the TCP bus side.
pub const TCP_MAX_OUTBOX: usize = 16;

/// Max raw UDS frame size for the CAN bus side
pub const CAN_MAX_FRAME: usize = 4096;

/// Max outbox depth on the CAN bus side.
pub const CAN_MAX_OUTBOX: usize = 16;

/// Max raw UDS frame size
pub const UDS_MAX_FRAME: usize = 4096;

/// Max outbox depth for UDS
pub const UDS_MAX_OUTBOX: usize = 8;

// endregion: Capacity Constants

// region: GatewayError

#[derive(Debug)]
pub enum GatewayError {
    /// DoIP header validation failed.
    InvalidDoipHeader,

    /// Codec encode/decode error.
    Codec,

    /// TCP outbox full.
    TcpOutboxFull,

    /// CAN outbox full.
    CanOutboxFull,

    /// No connection slot available for a new tester.
    NoConnectionSlot,

    /// ISO-TP Error
    IsoTpError(ace_can::IsoTpError),

    /// UDS outbox full.
    UdsOutboxFull,
}

// endregion: GatewayError

// region: EcuIsoTpNode

/// ISO-TP state for a single ECU - owned by the gateway.
///
/// The gateway owns one of these per registered ECU. All CAN framing for that ECU goes through
/// here. The scenario never touches ISO-TP directly - it only calls gateway methods.
struct EcuIsoTpNode<const N: usize = UDS_MAX_FRAME> {
    request_can_id: u32,
    response_can_id: u32,
    /// Segments UDS request bytes -> ISO-TP CAN frames for the ECU.
    req_segmenter: Segmenter<N>,

    /// Reassembles ISO-TP CAN response frames from the ECU -> UDS bytes.
    resp_reassembler: Reassembler<N>,
}

impl<const N: usize> EcuIsoTpNode<N> {
    fn new(request_can_id: u32, response_can_id: u32, mode: IsoTpAddressingMode) -> Self {
        Self {
            request_can_id,
            response_can_id,
            req_segmenter: Segmenter::new(SegmenterConfig::classic(mode.clone())),
            resp_reassembler: Reassembler::new(ReassemblerConfig::new(mode)),
        }
    }
}

// endregion: EcuIsoTpNode

// region: ConnectionSlot

/// A slot for one tester TCP connection.
struct ConnectionSlot<A: ActivationAuthProvider, const BUF: usize> {
    /// The logical address of the connected tester - `None` if slot is free.
    tester_address: Option<u16>,
    state: ConnectionState<A, BUF>,
}

// endregion: ConnectionSlot

// region: DoipGateway

pub struct DoipGateway<A, const MAX_TESTERS: usize = 1, const BUF: usize = 4096>
where
    A: ActivationAuthProvider + Clone,
{
    config: GatewayConfig,
    auth: A,
    address: NodeAddress,

    isotp_nodes: heapless::Vec<EcuIsoTpNode<BUF>, 8>,

    /// Outbound DoIP frames for the TCP bus.
    tcp_outbox: heapless::Vec<(NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,

    /// Outbound UDS bytes for the CAN bus (address by CAN request ID).
    can_outbox: heapless::Vec<(NodeAddress, heapless::Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,

    /// Pending route table - matches CAN responses to tester connections.
    routes: PendingRouteTable<MAX_TESTERS>,

    /// Active tester connection slots.
    connections: heapless::Vec<ConnectionSlot<A, BUF>, MAX_TESTERS>,
}

impl<A, const MAX_TESTERS: usize, const BUF: usize> DoipGateway<A, MAX_TESTERS, BUF>
where
    A: ActivationAuthProvider + Clone,
{
    pub fn new(config: GatewayConfig, auth: A, address: NodeAddress) -> Self {
        let mut isotp_nodes: heapless::Vec<EcuIsoTpNode<BUF>, 8> = heapless::Vec::new();
        for node in &config.nodes {
            let _ = isotp_nodes.push(EcuIsoTpNode::new(
                node.request_can_id,
                node.response_can_id,
                config.isotp_addressing_mode.clone(),
            ));
        }
        Self {
            config,
            auth,
            address,
            isotp_nodes,
            tcp_outbox: heapless::Vec::new(),
            can_outbox: heapless::Vec::new(),
            routes: PendingRouteTable::new(),
            connections: heapless::Vec::new(),
        }
    }

    // region: SimNode surface - TCP bus

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }

    /// Handles a raw DoIP frame from the TCP bus.
    ///
    /// Validates the header, dispatches on payload type, and drives the connection state machine
    /// for the originating tester.
    pub fn handle_tcp(
        &mut self,
        src: &NodeAddress,
        data: &[u8],
        now: Instant,
    ) -> Result<(), GatewayError> {
        let frame = DoipFrame::from_slice(data);

        if let Err(_) = frame.validate_header() {
            self.send_generic_nack(src)?;
            return Err(GatewayError::InvalidDoipHeader);
        }

        let payload_type = match frame.payload_type() {
            Some(Ok(pt)) => pt,
            _ => {
                self.send_generic_nack(src)?;
                return Err(GatewayError::InvalidDoipHeader);
            }
        };

        let payload_data = frame.payload_bytes().unwrap_or(&[]);

        let tester_address = src.0 as u16;
        self.ensure_slot(tester_address, now)?;

        let slot_idx = self.find_slot_idx(tester_address);

        if let Some(idx) = slot_idx {
            self.connections[idx]
                .state
                .handle(&payload_type, payload_data, now);
            self.process_connection_events(idx, src, now)?;
        }

        Ok(())
    }

    /// Handles a raw UDS response frame from the CAN bus (via IsoTpNode).
    ///
    /// The NodeAddress carries the CAN response ID so the router can match it to the originating
    /// tester.
    pub fn handle_can(
        &mut self,
        src: &NodeAddress,
        data: &[u8],
        _now: Instant,
    ) -> Result<(), GatewayError> {
        let can_id = src.0;

        if let Some(isotp) = self
            .isotp_nodes
            .iter_mut()
            .find(|n| n.request_can_id == can_id)
        {
            if let Err(e) = isotp.req_segmenter.handle_flow_control(data) {
                let _ = e;
            } else {
                let mut out_buf = [0u8; CAN_MAX_FRAME];
                let request_can_id = isotp.request_can_id;

                loop {
                    match isotp
                        .req_segmenter
                        .next_frame(&mut out_buf)
                        .map_err(GatewayError::IsoTpError)?
                    {
                        SegmentResult::Complete => break,
                        SegmentResult::WaitForFlowControl => break,
                        SegmentResult::Frame { len } => {
                            let mut frame: heapless::Vec<u8, CAN_MAX_FRAME> = heapless::Vec::new();
                            let _ = frame.extend_from_slice(&out_buf[..len]);

                            self.can_outbox
                                .push((NodeAddress(request_can_id), frame))
                                .map_err(|_| GatewayError::CanOutboxFull)?;
                        }
                    }
                }
            }
            return Ok(());
        }

        let response_can_id = can_id;
        let _node_cfg = match self
            .config
            .nodes
            .iter()
            .find(|n| n.response_can_id == response_can_id)
        {
            Some(n) => n.clone(),
            None => return Ok(()),
        };

        let mut pending: Option<(NodeAddress, u16, u16, heapless::Vec<u8, UDS_MAX_FRAME>)> = None;
        let mut fc_frame: Option<heapless::Vec<u8, CAN_MAX_FRAME>> = None;

        if let Some(isotp) = self
            .isotp_nodes
            .iter_mut()
            .find(|n| n.response_can_id == response_can_id)
        {
            match isotp
                .resp_reassembler
                .feed(data)
                .map_err(GatewayError::IsoTpError)?
            {
                ReassembleResult::Complete { len } => {
                    if let Some(uds_bytes) = isotp.resp_reassembler.message(len) {
                        if let Some(route) = self.routes.take_by_can_response_id(response_can_id) {
                            let mut buf: heapless::Vec<u8, UDS_MAX_FRAME> = heapless::Vec::new();
                            let _ = buf.extend_from_slice(&uds_bytes[..len.min(UDS_MAX_FRAME)]);

                            pending = Some((
                                NodeAddress(route.tester_address as u32),
                                route.doip_target,
                                route.doip_source,
                                buf,
                            ));
                        }
                    }
                }
                ReassembleResult::FlowControl { frame, len: fc_len } => {
                    let mut fc: heapless::Vec<u8, CAN_MAX_FRAME> = heapless::Vec::new();
                    let _ = fc.extend_from_slice(&frame[..fc_len]);

                    fc_frame = Some(fc);
                }
                ReassembleResult::SessionAborted {
                    flow_control,
                    fc_len,
                } => {
                    let mut fc: heapless::Vec<u8, CAN_MAX_FRAME> = heapless::Vec::new();
                    let _ = fc.extend_from_slice(&flow_control[..fc_len]);

                    fc_frame = Some(fc);
                    isotp.resp_reassembler.reset();
                }
                ReassembleResult::InProgress => {}
            }
        }

        if let Some(fc) = fc_frame {
            self.can_outbox
                .push((NodeAddress(response_can_id), fc))
                .map_err(|_| GatewayError::CanOutboxFull)?;
        }

        if let Some((dst, source, target, data)) = pending {
            self.send_diagnostic_message(&dst, source, target, &data)?;
        }

        Ok(())
    }

    pub fn tick(&mut self, now: Instant) -> Result<(), GatewayError> {
        for idx in 0..self.connections.len() {
            self.connections[idx].state.tick(now);

            let events: heapless::Vec<ConnectionEvent<BUF>, 8> =
                self.connections[idx].state.drain_events().collect();

            let tester_addr = self.connections[idx]
                .tester_address
                .map(|a| NodeAddress(a as u32))
                .unwrap_or(NodeAddress(0));

            for event in events {
                self.handle_connection_event(idx, &tester_addr, event)?;
            }
        }

        Ok(())
    }

    /// Drains outbound DoIP frames for the TCP bus.
    pub fn drain_tcp_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> usize {
        let n = self.tcp_outbox.len();

        for item in self.tcp_outbox.drain(..) {
            let _ = out.push(item);
        }

        n
    }

    /// Drains outbound UDS frames for the CAN bus.
    pub fn drain_can_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,
    ) -> usize {
        let n = self.can_outbox.len();

        for item in self.can_outbox.drain(..) {
            let _ = out.push(item);
        }

        n
    }

    // endregion: SimNode surface

    // region: Connection Slot Management

    fn find_slot_idx(&self, tester_address: u16) -> Option<usize> {
        self.connections
            .iter()
            .position(|s| s.tester_address == Some(tester_address))
    }

    fn ensure_slot(&mut self, tester_address: u16, now: Instant) -> Result<(), GatewayError> {
        if self.find_slot_idx(tester_address).is_some() {
            return Ok(());
        }

        if self.connections.is_full() {
            return Err(GatewayError::NoConnectionSlot);
        }

        let mut registered = heapless::Vec::new();

        for &addr in &self.config.registered_testers {
            let _ = registered.push(addr);
        }

        let mut supported = heapless::Vec::new();

        for t in &self.config.supported_activation_types {
            let _ = supported.push(t.clone());
        }

        let activation = ActivationStateMachine::new(
            self.config.logical_address,
            registered,
            supported,
            self.auth.clone(),
        );

        let state = ConnectionState::new(self.config.connection_config.clone(), activation, now);

        let _ = self.connections.push(ConnectionSlot {
            tester_address: Some(tester_address),
            state,
        });

        Ok(())
    }

    fn remove_slot(&mut self, tester_address: u16) {
        if let Some(pos) = self
            .connections
            .iter()
            .position(|s| s.tester_address == Some(tester_address))
        {
            self.connections.remove(pos);
            self.routes.remove_tester(tester_address);
        }
    }

    // endregion: Connection Slot Management

    // region: Event Processing

    fn process_connection_events(
        &mut self,
        slot_idx: usize,
        tester: &NodeAddress,
        _now: Instant,
    ) -> Result<(), GatewayError> {
        let events: heapless::Vec<ConnectionEvent<BUF>, 8> =
            self.connections[slot_idx].state.drain_events().collect();

        for event in events {
            self.handle_connection_event(slot_idx, tester, event)?;
        }

        Ok(())
    }

    fn handle_connection_event(
        &mut self,
        slot_idx: usize,
        tester: &NodeAddress,
        event: ConnectionEvent<BUF>,
    ) -> Result<(), GatewayError> {
        match event {
            ConnectionEvent::SendActivationResponse(resp) => {
                self.encode_and_send_tcp(tester, PayloadType::RoutingActivationResponse, &resp)?;
            }

            ConnectionEvent::SendDiagnosticAck {
                source_address,
                target_address,
            } => {
                let ack = DiagnosticMessageAck {
                    source_address: source_address.to_be_bytes(),
                    target_address: target_address.to_be_bytes(),
                    ack_code: DiagnosticAckCode::Acknowledged,
                    data: &[],
                };
                self.encode_and_send_tcp(tester, PayloadType::DiagnosticMessageAck, &ack)?;
            }

            ConnectionEvent::SendDiagnosticNack {
                source_address,
                target_address,
                nack_code,
            } => {
                let ack = DiagnosticMessageNack {
                    source_address: source_address.to_be_bytes(),
                    target_address: target_address.to_be_bytes(),
                    nack_code,
                };
                self.encode_and_send_tcp(tester, PayloadType::DiagnosticMessageNack, &ack)?;
            }

            ConnectionEvent::ForwardToEcu {
                source_address,
                target_address,
                uds_data,
            } => {
                let node = match self.config.find_node(target_address) {
                    Some(n) => n.clone(),
                    None => {
                        let nack = DiagnosticMessageNack {
                            source_address: source_address.to_be_bytes(),
                            target_address: target_address.to_be_bytes(),
                            nack_code: DiagnosticNackCode::UnknownTargetAddress,
                        };

                        return self.encode_and_send_tcp(
                            tester,
                            PayloadType::DiagnosticMessageNack,
                            &nack,
                        );
                    }
                };

                let tester_address = self.connections[slot_idx].tester_address.unwrap_or(0);
                let _ = self.routes.insert(PendingRoute {
                    tester_address,
                    doip_source: source_address,
                    doip_target: target_address,
                    response_can_id: node.response_can_id,
                });

                let isotp = match self
                    .isotp_nodes
                    .iter_mut()
                    .find(|n| n.request_can_id == node.request_can_id)
                {
                    Some(n) => n,
                    None => return Ok(()),
                };

                isotp
                    .req_segmenter
                    .start(&uds_data)
                    .map_err(GatewayError::IsoTpError)?;

                let mut out_buf = [0u8; CAN_MAX_FRAME];
                let request_can_id = node.request_can_id;

                loop {
                    match isotp
                        .req_segmenter
                        .next_frame(&mut out_buf)
                        .map_err(GatewayError::IsoTpError)?
                    {
                        SegmentResult::Complete => break,
                        SegmentResult::WaitForFlowControl => break,
                        SegmentResult::Frame { len } => {
                            let mut frame: heapless::Vec<u8, CAN_MAX_FRAME> = heapless::Vec::new();
                            let _ = frame.extend_from_slice(&out_buf[..len]);

                            self.can_outbox
                                .push((NodeAddress(request_can_id), frame))
                                .map_err(|_| GatewayError::CanOutboxFull)?;
                        }
                    }
                }
            }

            ConnectionEvent::SendAliveCheckRequest => {
                let req = AliveCheckRequest {};

                self.encode_and_send_tcp(tester, PayloadType::AliveCheckRequest, &req)?;
            }

            ConnectionEvent::SendAliveCheckResponse => {
                let tester_address = self.connections[slot_idx].tester_address.unwrap_or(0);

                let resp = AliveCheckResponse {
                    source_address: tester_address.to_be_bytes(),
                };

                self.encode_and_send_tcp(tester, PayloadType::AliveCheckResponse, &resp)?;
            }

            ConnectionEvent::Close => {
                let tester_address = self.connections[slot_idx].tester_address.unwrap_or(0);

                self.remove_slot(tester_address);
            }
        }

        Ok(())
    }

    // endregion: Event Processing

    // region: Frame Construction Helpers

    fn encode_and_send_tcp<T: FrameWrite<Error = DoipError>>(
        &mut self,
        dst: &NodeAddress,
        payload_type: PayloadType,
        payload: &T,
    ) -> Result<(), GatewayError> {
        let mut payload_staging = [0u8; TCP_MAX_FRAME];
        let mut payload_slice: &mut [u8] = &mut payload_staging;

        payload
            .encode(&mut payload_slice)
            .map_err(|_| GatewayError::Codec)?;

        let payload_len = TCP_MAX_FRAME - payload_slice.len();

        let version_byte = self.config.protocol_version as u8;

        let header = DoipHeader {
            protocol_version: self.config.protocol_version,
            inverse_protocol_version: !version_byte,
            payload_type,
            payload_length: payload_len as u32,
        };

        let mut header_staging = [0u8; 8];
        let mut header_slice: &mut [u8] = &mut header_staging;

        header
            .encode(&mut header_slice)
            .map_err(|_| GatewayError::Codec)?;

        let header_len = 8 - header_slice.len();

        let mut frame: heapless::Vec<u8, TCP_MAX_FRAME> = heapless::Vec::new();

        frame
            .extend_from_slice(&header_staging[..header_len])
            .map_err(|_| GatewayError::Codec)?;

        frame
            .extend_from_slice(&payload_staging[..payload_len])
            .map_err(|_| GatewayError::Codec)?;

        self.tcp_outbox
            .push((dst.clone(), frame))
            .map_err(|_| GatewayError::TcpOutboxFull)
    }

    fn send_diagnostic_message(
        &mut self,
        dst: &NodeAddress,
        source_address: u16,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<(), GatewayError> {
        let payload_len = 4 + uds_data.len();

        let header = DoipHeader {
            protocol_version: ProtocolVersion::Iso13400_2012,
            inverse_protocol_version: !(ProtocolVersion::Iso13400_2012 as u8),
            payload_type: PayloadType::DiagnosticMessage,
            payload_length: payload_len as u32,
        };

        let mut frame: heapless::Vec<u8, TCP_MAX_FRAME> = heapless::Vec::new();
        let mut header_staging = [0u8; 8];
        {
            let mut slice: &mut [u8] = &mut header_staging;
            header.encode(&mut slice).map_err(|_| GatewayError::Codec)?;
            let written = 8 - slice.len();
            let _ = frame.extend_from_slice(&header_staging[..written]);
        }
        let _ = frame.extend_from_slice(&source_address.to_be_bytes());
        let _ = frame.extend_from_slice(&target_address.to_be_bytes());
        let _ = frame.extend_from_slice(uds_data);

        self.tcp_outbox
            .push((dst.clone(), frame))
            .map_err(|_| GatewayError::TcpOutboxFull)
    }

    fn send_generic_nack(&mut self, dst: &NodeAddress) -> Result<(), GatewayError> {
        let nack = GenericNack {
            nack_code: NackCode::InvalidPayloadLength,
        };

        self.encode_and_send_tcp(dst, PayloadType::GenericNack, &nack)
    }

    // endregion: Frame Construction Helpers
}

// endregion: DoipGateway
