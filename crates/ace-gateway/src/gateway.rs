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
use ace_core::storage::CapacityError;
use ace_core::{FrameWrite, Vec};
use ace_core::{
    clock::Instant,
    io::{InboundFrame, NodeAddress},
};
use ace_doip::{
    error::DoipError,
    ext::DoipFrameExt,
    header::{DoipHeader, PayloadType, ProtocolVersion},
    payload::{
        AliveCheckRequest, AliveCheckResponse, DiagnosticAckCode, DiagnosticMessageAck,
        DiagnosticMessageNack, DiagnosticNackCode, GenericNack, NackCode,
    },
    session::{ActivationAuthProvider, ActivationStateMachine, ConnectionAction, ConnectionState},
};
use ace_proto::{DoipFrame, LogicalAddress};

use crate::{
    config::GatewayConfig,
    router::{PendingRoute, PendingRouteTable},
};

// endregion: Imports

// region: Capacity Constants

// /// Max DoIP frame size for the TCP bus side.
// pub const TCP_MAX_FRAME: usize = 4096; // UDS + DoIP header + addresses
//
// /// Max outbox depth on the TCP bus side.
// pub const TCP_MAX_OUTBOX: usize = 8;
//
// /// Max raw UDS frame size for the CAN bus side
// pub const CAN_MAX_FRAME: usize = 4096;
//
// /// Max outbox depth on the CAN bus side.
// pub const CAN_MAX_OUTBOX: usize = 16;
//
// /// Max raw UDS frame size
// pub const UDS_MAX_FRAME: usize = 4096;
//
// /// Max outbox depth for UDS
// pub const UDS_MAX_OUTBOX: usize = 8;

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

    /// An assembled frame exceeded its bounded buffer. Distinct from `Codec`, which
    /// is a wire-format failure: this one says the buffer is too small.
    FrameTooLong,

    /// The per-connection DoIP state machine reported a failure.
    ///
    /// Wrapped rather than flattened: the gateway drives `ConnectionState` and a
    /// caller needs to tell a connection-level fault from a gateway-level one.
    Doip(ace_doip::error::DoipError),

    /// The bounded action buffer could not hold everything drained from a
    /// connection. Reported rather than truncated: dropping a pending action would
    /// desynchronise the gateway from the connection state machine.
    ActionBufferOverflow,

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
struct EcuIsoTpNode<const ISOTP_MAX_FRAME: usize> {
    request_can_id: u32,
    response_can_id: u32,
    /// Segments UDS request bytes -> ISO-TP CAN frames for the ECU.
    req_segmenter: Segmenter<ISOTP_MAX_FRAME>,

    /// Reassembles ISO-TP CAN response frames from the ECU -> UDS bytes.
    resp_reassembler: Reassembler<ISOTP_MAX_FRAME>,

    /// Kept so the single-frame payload limit can be derived for broadcasts.
    addressing_mode: IsoTpAddressingMode,
}

impl<const ISOTP_MAX_FRAME: usize> EcuIsoTpNode<ISOTP_MAX_FRAME> {
    /// Largest payload this node can carry in one unsegmented frame.
    fn max_single_frame_payload(&self) -> usize {
        self.addressing_mode.max_sf_payload_classic()
    }

    fn new(request_can_id: u32, response_can_id: u32, mode: IsoTpAddressingMode) -> Self {
        Self {
            request_can_id,
            response_can_id,
            req_segmenter: Segmenter::new(SegmenterConfig::classic(mode.clone())),
            resp_reassembler: Reassembler::new(ReassemblerConfig::new(mode.clone())),
            addressing_mode: mode,
        }
    }
}

// endregion: EcuIsoTpNode

// region: ConnectionSlot

/// A slot for one tester TCP connection.
struct ConnectionSlot<
    // region: ConnectionState Constants
    const UDS_MAX_FRAME: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const MAX_TESTERS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    // endregion: ConnectionState Constants
    A: ActivationAuthProvider,
> {
    /// The logical address of the connected tester - `None` if slot is free.
    tester_address: Option<u16>,
    state: ConnectionState<
        UDS_MAX_FRAME,
        MAX_CONNECTION_ACTIONS,
        MAX_TESTERS,
        MAX_ACTIVATION_TYPES,
        A,
    >,
}

// endregion: ConnectionSlot

// region: DoipGateway

pub struct DoipGateway<
    const CAN_MAX_FRAME: usize,
    const ISOTP_MAX_NODES: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const CAN_MAX_OUTBOX: usize,
    const MAX_TESTERS: usize,
    const MAX_FRAME: usize,
    const UDS_MAX_FRAME: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    const MAX_NODES: usize,
    A,
> where
    A: ActivationAuthProvider + Clone,
{
    config: GatewayConfig<MAX_NODES, MAX_TESTERS, MAX_ACTIVATION_TYPES>,
    auth: A,
    address: NodeAddress,

    isotp_nodes: Vec<EcuIsoTpNode<CAN_MAX_FRAME>, ISOTP_MAX_NODES>,

    /// Outbound DoIP frames for the TCP bus.
    tcp_outbox: Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,

    /// Outbound UDS bytes for the CAN bus (address by CAN request ID).
    can_outbox: Vec<(NodeAddress, Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,

    /// Pending route table - matches CAN responses to tester connections.
    ///
    /// Sized by node count rather than tester count. One functionally addressed request
    /// needs a route per responding ECU, so a table sized by testers could not hold a
    /// single fan-out. Concurrent fan-outs from several testers can still exhaust it,
    /// which is answered with a NACK rather than silently dropping a route - stable Rust
    /// cannot express `MAX_TESTERS * ISOTP_MAX_NODES` in a type position.
    routes: PendingRouteTable<ISOTP_MAX_NODES>,

    /// Active tester connection slots.
    connections: Vec<
        ConnectionSlot<UDS_MAX_FRAME, MAX_CONNECTION_ACTIONS, MAX_TESTERS, MAX_ACTIVATION_TYPES, A>,
        MAX_TESTERS,
    >,
}

impl<
    const CAN_MAX_FRAME: usize,
    const ISOTP_MAX_NODES: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const CAN_MAX_OUTBOX: usize,
    const MAX_TESTERS: usize,
    const MAX_FRAME: usize,
    const UDS_MAX_FRAME: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    const MAX_NODES: usize,
    A,
>
    DoipGateway<
        CAN_MAX_FRAME,
        ISOTP_MAX_NODES,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        CAN_MAX_OUTBOX,
        MAX_TESTERS,
        MAX_FRAME,
        UDS_MAX_FRAME,
        MAX_CONNECTION_ACTIONS,
        MAX_ACTIVATION_TYPES,
        MAX_NODES,
        A,
    >
where
    A: ActivationAuthProvider + Clone,
{
    pub fn new(
        config: GatewayConfig<MAX_NODES, MAX_TESTERS, MAX_ACTIVATION_TYPES>,
        auth: A,
        address: NodeAddress,
    ) -> Self {
        let mut isotp_nodes: Vec<EcuIsoTpNode<CAN_MAX_FRAME>, ISOTP_MAX_NODES> = Vec::new();

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
            tcp_outbox: Vec::new(),
            can_outbox: Vec::new(),
            routes: PendingRouteTable::new(),
            connections: Vec::new(),
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
    /// Handles a DoIP frame seen on the TCP segment.
    ///
    /// Ignores frames addressed elsewhere. Previously this took only a source address, so
    /// it had no way to tell whether a frame was for it and whatever drove the gateway had
    /// to decide - which meant the destination check lived outside the library.
    pub fn handle_tcp(
        &mut self,
        inbound: &InboundFrame<'_>,
        now: Instant,
    ) -> Result<(), GatewayError> {
        if inbound.dst.0 != self.address.0 {
            return Ok(());
        }

        let src = &inbound.src;
        let data = inbound.data;
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
                .handle(&payload_type, payload_data, now)
                .map_err(GatewayError::Doip)?;
            self.process_connection_actions(idx, src, now)?;
        }

        Ok(())
    }

    /// Handles a raw UDS response frame from the CAN bus (via IsoTpNode).
    ///
    /// The NodeAddress carries the CAN response ID so the router can match it to the originating
    /// tester.
    /// Handles a CAN frame seen on the segment.
    ///
    /// Takes every frame on the bus and decides for itself which are its own, rather than
    /// relying on the caller to route. The dispatch below is by arrival identifier: a
    /// frame on an ECU's request identifier is that ECU's flow control, and one on its
    /// response identifier is response payload.
    pub fn handle_can_frame(
        &mut self,
        frame: &InboundFrame<'_>,
        _now: Instant,
    ) -> Result<(), GatewayError> {
        // A CAN controller does not accept its own transmissions. The gateway sends
        // requests on the very identifiers it reads flow control from, so without this
        // its own outgoing frames would be fed back into its own request segmenter.
        //
        // Deliberately not load-bearing: removing it currently fails no test, because
        // `handle_flow_control` rejects a data frame and that error is discarded just
        // below. Stating the rule here does not depend on that.
        if frame.src.0 == self.address.0 {
            return Ok(());
        }

        let data = frame.data;
        let can_id = frame.dst.0;

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
                            let mut frame: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                            frame
                                .extend_from_slice(&out_buf[..len])
                                .map_err(|_| GatewayError::FrameTooLong)?;

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

        let mut pending: Option<(NodeAddress, u16, u16, Vec<u8, UDS_MAX_FRAME>)> = None;
        let mut fc_frame: Option<Vec<u8, CAN_MAX_FRAME>> = None;

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
                            let mut buf: Vec<u8, UDS_MAX_FRAME> = Vec::new();
                            buf.extend_from_slice(&uds_bytes[..len])
                                .map_err(|_| GatewayError::FrameTooLong)?;

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
                    let mut fc: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                    fc.extend_from_slice(&frame[..fc_len])
                        .map_err(|_| GatewayError::FrameTooLong)?;

                    fc_frame = Some(fc);
                }
                ReassembleResult::SessionAborted {
                    flow_control,
                    fc_len,
                } => {
                    let mut fc: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                    fc.extend_from_slice(&flow_control[..fc_len])
                        .map_err(|_| GatewayError::FrameTooLong)?;

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
            self.connections[idx]
                .state
                .tick(now)
                .map_err(GatewayError::Doip)?;

            let actions: Vec<ConnectionAction<UDS_MAX_FRAME>, MAX_CONNECTION_ACTIONS> =
                Vec::from_iter_checked(self.connections[idx].state.drain_actions())
                    .map_err(|_| GatewayError::ActionBufferOverflow)?;

            let tester_addr = self.connections[idx]
                .tester_address
                .map(|a| NodeAddress(a as u32))
                .unwrap_or(NodeAddress(0));

            for action in actions {
                self.handle_connection_action(idx, &tester_addr, action)?;
            }
        }

        Ok(())
    }

    /// Drains outbound DoIP frames for the TCP bus.
    pub fn drain_tcp_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> Result<usize, CapacityError> {
        // Refused before draining rather than during: the source is emptied as it is
        // read, so a partial move would lose the remainder with no way to recover it.
        if self.tcp_outbox.len() > out.capacity() - out.len() {
            return Err(CapacityError);
        }

        let n = self.tcp_outbox.len();

        for item in self.tcp_outbox.drain(..) {
            // Cannot fail: capacity checked above.
            let _ = out.push(item);
        }

        Ok(n)
    }

    /// Drains outbound UDS frames for the CAN bus.
    pub fn drain_can_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,
    ) -> Result<usize, CapacityError> {
        // Refused before draining rather than during: the source is emptied as it is
        // read, so a partial move would lose the remainder with no way to recover it.
        if self.can_outbox.len() > out.capacity() - out.len() {
            return Err(CapacityError);
        }

        let n = self.can_outbox.len();

        for item in self.can_outbox.drain(..) {
            // Cannot fail: capacity checked above.
            let _ = out.push(item);
        }

        Ok(n)
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

        if self.connections.len() >= MAX_TESTERS {
            return Err(GatewayError::NoConnectionSlot);
        }

        let mut registered = Vec::new();

        for &addr in &self.config.registered_testers {
            let _ = registered.push(addr);
        }

        let mut supported = Vec::new();

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

    fn process_connection_actions(
        &mut self,
        slot_idx: usize,
        tester: &NodeAddress,
        _now: Instant,
    ) -> Result<(), GatewayError> {
        let actions: Vec<ConnectionAction<UDS_MAX_FRAME>, MAX_CONNECTION_ACTIONS> =
            Vec::from_iter_checked(self.connections[slot_idx].state.drain_actions())
                .map_err(|_| GatewayError::ActionBufferOverflow)?;

        for action in actions {
            self.handle_connection_action(slot_idx, tester, action)?;
        }

        Ok(())
    }

    /// Routes a diagnostic request, returning `None` when it was forwarded or the code
    /// to refuse it with.
    ///
    /// A refusal is a protocol answer, not an error: the tester is told why, and the
    /// gateway carries on serving other connections.
    fn forward_request(
        &mut self,
        slot_idx: usize,
        source_address: u16,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<Option<DiagnosticNackCode>, GatewayError> {
        if LogicalAddress::new(target_address).is_functional() {
            self.fan_out(slot_idx, source_address, target_address, uds_data)
        } else {
            self.forward_to_one(slot_idx, source_address, target_address, uds_data)
        }
    }

    /// Forwards a physically addressed request to the single ECU that owns the address.
    fn forward_to_one(
        &mut self,
        slot_idx: usize,
        source_address: u16,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<Option<DiagnosticNackCode>, GatewayError> {
        let node = match self.config.find_node(target_address) {
            Some(n) => n.clone(),
            None => return Ok(Some(DiagnosticNackCode::UnknownTargetAddress)),
        };

        let tester_address = self.connections[slot_idx].tester_address.unwrap_or(0);

        // Checked before committing, and reported rather than dropped. This push
        // previously discarded its result, so a full table silently produced a request
        // whose response could never be matched back to the tester that asked for it.
        if self.routes.remaining() == 0 {
            return Ok(Some(DiagnosticNackCode::OutOfMemory));
        }

        if !self.routes.insert(PendingRoute {
            tester_address,
            doip_source: source_address,
            doip_target: target_address,
            response_can_id: node.response_can_id,
        }) {
            return Ok(Some(DiagnosticNackCode::OutOfMemory));
        }

        self.segment_onto_can(node.request_can_id, node.request_can_id, uds_data)?;

        Ok(None)
    }

    /// Forwards a functionally addressed request to every ECU behind this gateway.
    ///
    /// All-or-nothing. Every capacity question is settled before anything is committed,
    /// because a partial fan-out followed by an acknowledgement would tell the tester
    /// that all ECUs received a request most of them never saw - which is worse than a
    /// refusal, since the tester cannot tell the difference between an ECU that declined
    /// to answer and one that was never asked.
    ///
    /// Overflow is answered with `OutOfMemory` rather than raised as an error: DoIP
    /// already defines the response for a gateway that cannot accept a message, and
    /// where the protocol defines an answer the protocol wins.
    fn fan_out(
        &mut self,
        slot_idx: usize,
        source_address: u16,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<Option<DiagnosticNackCode>, GatewayError> {
        let node_count = self.config.nodes.len();

        if node_count == 0 {
            return Ok(Some(DiagnosticNackCode::UnknownTargetAddress));
        }

        // One route per ECU: each answers on its own response identifier, so each needs
        // a slot for its reply to be matched back to this tester.
        //
        // The sizing precondition is that the route table can hold a whole fan-out.
        // Stable Rust cannot express a relation between two const generic parameters, so
        // it is asserted in debug builds and answered with a NACK at runtime.
        debug_assert!(
            self.routes.capacity() >= node_count,
            "route table is too small to hold a fan-out to every ECU"
        );

        if self.routes.remaining() < node_count {
            return Ok(Some(DiagnosticNackCode::OutOfMemory));
        }

        // A functional request is one transmission on a shared identifier, so it has to
        // fit a single frame. ISO 15765-2 segmentation cannot be used: a first frame
        // would draw a flow-control response from every receiver at once, and the sender
        // has no way to tell whose to obey.
        let node = self.config.nodes[0].clone();

        let single_frame_limit = match self
            .isotp_nodes
            .iter()
            .find(|n| n.request_can_id == node.request_can_id)
        {
            Some(n) => n.max_single_frame_payload(),
            None => return Ok(Some(DiagnosticNackCode::TargetUnreachable)),
        };

        if uds_data.len() > single_frame_limit {
            return Ok(Some(DiagnosticNackCode::TransportProtocolError));
        }

        if self.can_outbox.is_full() {
            return Ok(Some(DiagnosticNackCode::OutOfMemory));
        }

        let tester_address = self.connections[slot_idx].tester_address.unwrap_or(0);

        // Committed only now that every check has passed.
        for entry in self.config.nodes.iter() {
            // Each route carries the ECU's own logical address, not the functional one:
            // a stored route's target becomes the source of the response sent back, so
            // reusing the group address here would make every reply claim to come from
            // the group rather than from the ECU that produced it.
            let inserted = self.routes.insert(PendingRoute {
                tester_address,
                doip_source: source_address,
                doip_target: entry.logical_address,
                response_can_id: entry.response_can_id,
            });

            debug_assert!(
                inserted,
                "capacity was checked before committing the fan-out"
            );
        }

        // One frame on the shared identifier, not one per ECU - that is what makes this
        // a broadcast rather than a loop of unicasts.
        debug_assert!(
            self.config
                .nodes
                .iter()
                .all(|n| n.functional_can_id == node.functional_can_id),
            "a broadcast presumes one shared functional identifier on the segment"
        );

        self.segment_onto_can(node.request_can_id, node.functional_can_id, uds_data)?;

        let _ = target_address;

        Ok(None)
    }

    /// Segments UDS bytes with `segmenter_can_id`'s ISO-TP state and sends them on
    /// `send_can_id`.
    ///
    /// The two differ for a broadcast: the segmenter belongs to a specific ECU, but the
    /// frames go out on the shared functional identifier.
    fn segment_onto_can(
        &mut self,
        segmenter_can_id: u32,
        send_can_id: u32,
        uds_data: &[u8],
    ) -> Result<(), GatewayError> {
        let isotp = match self
            .isotp_nodes
            .iter_mut()
            .find(|n| n.request_can_id == segmenter_can_id)
        {
            Some(n) => n,
            None => return Ok(()),
        };

        isotp
            .req_segmenter
            .start(uds_data)
            .map_err(GatewayError::IsoTpError)?;

        let mut out_buf = [0u8; CAN_MAX_FRAME];
        let mut staged: Vec<(NodeAddress, Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX> = Vec::new();

        loop {
            match isotp
                .req_segmenter
                .next_frame(&mut out_buf)
                .map_err(GatewayError::IsoTpError)?
            {
                SegmentResult::Complete => break,
                SegmentResult::WaitForFlowControl => break,
                SegmentResult::Frame { len } => {
                    let mut frame: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                    frame
                        .extend_from_slice(&out_buf[..len])
                        .map_err(|_| GatewayError::FrameTooLong)?;

                    staged
                        .push((NodeAddress(send_can_id), frame))
                        .map_err(|_| GatewayError::CanOutboxFull)?;
                }
            }
        }

        for frame in staged {
            self.can_outbox
                .push(frame)
                .map_err(|_| GatewayError::CanOutboxFull)?;
        }

        Ok(())
    }

    fn handle_connection_action(
        &mut self,
        slot_idx: usize,
        tester: &NodeAddress,
        action: ConnectionAction<UDS_MAX_FRAME>,
    ) -> Result<(), GatewayError> {
        match action {
            ConnectionAction::SendActivationResponse(resp) => {
                self.encode_and_send_tcp(tester, PayloadType::RoutingActivationResponse, &resp)?;
            }

            ConnectionAction::SendDiagnosticNack {
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

            ConnectionAction::ForwardToEcu {
                source_address,
                target_address,
                uds_data,
            } => {
                // Decide first, then answer. A DoIP acknowledgement states the request
                // was accepted for processing, which is only knowable once routing has
                // been established - so the ack is this gateway's answer to the forward
                // rather than something promised ahead of it.
                let refusal =
                    self.forward_request(slot_idx, source_address, target_address, &uds_data)?;

                let gateway_address = self.config.logical_address;

                match refusal {
                    None => {
                        let ack = DiagnosticMessageAck {
                            source_address: gateway_address.to_be_bytes(),
                            target_address: source_address.to_be_bytes(),
                            ack_code: DiagnosticAckCode::Acknowledged,
                            data: &[],
                        };
                        self.encode_and_send_tcp(tester, PayloadType::DiagnosticMessageAck, &ack)?;
                    }
                    Some(nack_code) => {
                        let nack = DiagnosticMessageNack {
                            source_address: gateway_address.to_be_bytes(),
                            target_address: source_address.to_be_bytes(),
                            nack_code,
                        };
                        self.encode_and_send_tcp(
                            tester,
                            PayloadType::DiagnosticMessageNack,
                            &nack,
                        )?;
                    }
                }
            }

            ConnectionAction::SendAliveCheckRequest => {
                let req = AliveCheckRequest {};

                self.encode_and_send_tcp(tester, PayloadType::AliveCheckRequest, &req)?;
            }

            ConnectionAction::SendAliveCheckResponse => {
                let tester_address = self.connections[slot_idx].tester_address.unwrap_or(0);

                let resp = AliveCheckResponse {
                    source_address: tester_address.to_be_bytes(),
                };

                self.encode_and_send_tcp(tester, PayloadType::AliveCheckResponse, &resp)?;
            }

            ConnectionAction::Close => {
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

        let mut frame: Vec<u8, TCP_MAX_FRAME> = Vec::new();

        frame
            .extend_from_slice(&header_staging[..header_len])
            .map_err(|_| GatewayError::Codec)?;

        frame
            .extend_from_slice(&payload_staging[..payload_len])
            .map_err(|_| GatewayError::Codec)?;

        self.tcp_outbox
            .push((dst.clone(), frame))
            .map_err(|_| GatewayError::TcpOutboxFull)?;

        Ok(())
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

        let mut frame: Vec<u8, TCP_MAX_FRAME> = Vec::new();
        let mut header_staging = [0u8; 8];
        {
            let mut slice: &mut [u8] = &mut header_staging;
            header.encode(&mut slice).map_err(|_| GatewayError::Codec)?;
            let written = 8 - slice.len();
            frame
                .extend_from_slice(&header_staging[..written])
                .map_err(|_| GatewayError::FrameTooLong)?;
        }
        frame
            .extend_from_slice(&source_address.to_be_bytes())
            .map_err(|_| GatewayError::FrameTooLong)?;
        frame
            .extend_from_slice(&target_address.to_be_bytes())
            .map_err(|_| GatewayError::FrameTooLong)?;
        frame
            .extend_from_slice(uds_data)
            .map_err(|_| GatewayError::Codec)?;

        self.tcp_outbox
            .push((dst.clone(), frame))
            .map_err(|_| GatewayError::TcpOutboxFull)?;

        Ok(())
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
