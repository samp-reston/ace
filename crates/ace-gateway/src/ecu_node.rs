//! EcuNode - the ECU-side ISO-TP + UDS application layer.
//!
//! In a real vehicle the ECU contains:
//!   - A CAN transceiver receiving raw CAN frames
//!   - An ISO-TP stack reassembling multi-frame requests and segmenting responses
//!   - A UDS application layer (UdsServer) processing the reassembled requests
//!
//! EcuNode models exactly this. The gateway puts ISO-TP CAN frames onto the
//! CAN bus addressed to request_can_id. EcuNode reassembles them, feeds the
//! UDS bytes to UdsServer, and segments the response back into ISO-TP CAN
//! frames addressed to response_can_id. The gateway's resp_reassembler then
//! picks those up from the CAN bus and wraps them in DoIP for the tester.
//!
//! Flow control is exchanged naturally via the CAN bus:
//!   Gateway req_segmenter sends FF → EcuNode req_reassembler sends FC back
//!   EcuNode resp_segmenter sends FF → Gateway resp_reassembler sends FC back

use ace_can::{
    IsoTpAddressingMode, ReassembleResult, Reassembler, ReassemblerConfig, SegmentResult,
    Segmenter, SegmenterConfig,
};
use ace_core::Vec;
use ace_core::storage::CapacityError;
use ace_core::{
    clock::Instant,
    io::{InboundFrame, NodeAddress},
};
use ace_server::{
    NrcError,
    handler::ServerHandler,
    security_provider::SecurityProvider,
    server::{ServerError, UdsServer},
};

/// What a frame arriving on the CAN segment means to an ECU node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanArrival {
    /// A request addressed to this node alone.
    PhysicalRequest,

    /// A request broadcast to every node on the segment.
    FunctionalRequest,

    /// The peer's flow control for a multi-frame response this node is sending.
    FlowControlForOurResponse,
}

// region: EcuNodeError

#[derive(Debug)]
pub enum EcuNodeError<SE: NrcError> {
    IsoTp(ace_can::IsoTpError),
    CanOutboxFull,
    Server(ServerError<SE>),
    /// An assembled frame exceeded its bounded buffer.
    FrameTooLong,
}

// endregion: EcuNodeError

// region: EcuNode

/// ECU-side ISO-TP + UDS application node.
///
/// Sits on the CAN simulation bus. Receives ISO-TP CAN frames from the
/// gateway, reassembles them into UDS requests, processes them with
/// `UdsServer`, then segments the responses back into ISO-TP CAN frames
/// for the gateway to pick up and wrap in DoIP.
pub struct EcuNode<
    const UDS_MAX_FRAME: usize,
    const UDS_MAX_OUTBOX: usize,
    const CAN_MAX_FRAME: usize,
    const CAN_MAX_OUTBOX: usize,
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
    /// DoIP logical address of this ECU.
    pub logical_address: u16,

    /// CAN ID the gateway sends requests to.
    pub request_can_id: u32,

    /// CAN ID the gateway listens on for responses.
    pub response_can_id: u32,

    /// CAN ID carrying functionally addressed (broadcast) requests.
    pub functional_can_id: u32,

    /// Reassembles incoming ISO-TP CAN request frames → UDS bytes.
    req_reassembler: Reassembler<CAN_MAX_FRAME>,

    /// Segments outgoing UDS response bytes → ISO-TP CAN frames.
    resp_segmenter: Segmenter<CAN_MAX_FRAME>,
    pub server: UdsServer<
        UDS_MAX_FRAME,
        UDS_MAX_OUTBOX,
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
    >,

    /// Outbound ISO-TP CAN frames for the CAN bus.
    /// Contains both FC frames (responding to gateway segmenter) and
    /// segmented response frames.
    can_outbox: Vec<(NodeAddress, Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,
}

impl<
    const UDS_MAX_FRAME: usize,
    const UDS_MAX_OUTBOX: usize,
    const CAN_MAX_FRAME: usize,
    const CAN_MAX_OUTBOX: usize,
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
    EcuNode<
        UDS_MAX_FRAME,
        UDS_MAX_OUTBOX,
        CAN_MAX_FRAME,
        CAN_MAX_OUTBOX,
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
    /// Builds an ECU node.
    ///
    /// `functional_can_id` is the CAN identifier carrying functionally addressed
    /// requests - 0x7DF by convention on classic CAN. Frames arriving there are
    /// translated to the server's functional UDS address rather than its physical one,
    /// which is what lets the server apply ISO 14229-1's suppression rules without
    /// knowing anything about CAN.
    pub fn new(
        logical_address: u16,
        request_can_id: u32,
        response_can_id: u32,
        functional_can_id: u32,
        mode: IsoTpAddressingMode,
        server: UdsServer<
            UDS_MAX_FRAME,
            UDS_MAX_OUTBOX,
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
        >,
    ) -> Self {
        Self {
            logical_address,
            request_can_id,
            response_can_id,
            functional_can_id,
            req_reassembler: Reassembler::new(ReassemblerConfig::new(mode.clone())),
            resp_segmenter: Segmenter::new(SegmenterConfig::classic(mode)),
            server,
            can_outbox: Vec::new(),
        }
    }

    // region: CAN frame input

    /// Receives an ISO-TP CAN frame from the CAN bus.
    ///
    /// Called by the tick when a CAN frame is addressed to `request_can_id`.
    /// Feeds into the request reassembler. On complete UDS message, calls
    /// `server.handle`. On flow control needed, emits FC frame to `can_outbox`.
    /// Also handles FC frames arriving from the gateway (responding to our
    /// response segmenter's multi-frame transmissions).
    /// Delivers a CAN frame that arrived on this node's bus.
    ///
    /// Takes the whole frame rather than just its bytes so the arrival CAN ID survives
    /// to the UDS layer. On classic CAN the functional address *is* a distinct CAN ID -
    /// 0x7DF conventionally - so discarding the arrival ID here would make a broadcast
    /// indistinguishable from a physically addressed request, and node-side addressing
    /// impossible to implement above this point.
    /// Decides whether a frame on the segment is this node's, and what it means.
    ///
    /// `None` is the common case on a shared bus: most frames belong to somebody else.
    fn classify_arrival(&self, frame: &InboundFrame<'_>) -> Option<CanArrival> {
        // A CAN controller does not accept its own transmissions. This node's flow
        // control is addressed to its request identifier, so without this it would come
        // back and be classified as a fresh request.
        //
        // Deliberately not load-bearing: removing it currently fails no test, because a
        // flow-control PCI fed to the request reassembler is rejected and the error is
        // discarded by the caller. That makes the correctness of this layer depend on a
        // swallowed error one layer down, which is what this states explicitly instead.
        if frame.src.0 == self.response_can_id {
            return None;
        }

        // Flow control is the one PCI that travels against the direction of data, which
        // is what distinguishes it from a request arriving on the same identifier.
        let is_flow_control = frame.data.first().is_some_and(|first| first & 0xF0 == 0x30);

        if is_flow_control && frame.dst.0 == self.response_can_id {
            return Some(CanArrival::FlowControlForOurResponse);
        }

        if frame.dst.0 == self.request_can_id {
            return Some(CanArrival::PhysicalRequest);
        }

        if frame.dst.0 == self.functional_can_id {
            return Some(CanArrival::FunctionalRequest);
        }

        None
    }

    pub fn handle_can_frame(
        &mut self,
        frame: &InboundFrame<'_>,
        now: Instant,
    ) -> Result<(), EcuNodeError<H::Error>> {
        let data = frame.data;

        // Acceptance is decided here, before any ISO-TP state is touched.
        //
        // This decision used to sit at the far end of the function: flow control was fed
        // to the response segmenter and payload to the request reassembler first, and
        // only then was the destination checked. Every frame on the segment therefore
        // mutated this node's state even when it belonged to another ECU, and the node
        // was correct only because whatever drove it filtered first - which a real ECU
        // has nobody to do for it.
        let arrival = match self.classify_arrival(frame) {
            Some(a) => a,
            None => return Ok(()),
        };

        if matches!(arrival, CanArrival::FlowControlForOurResponse) {
            if self.resp_segmenter.handle_flow_control(data).is_err() {
                // Non-fatal: the segmenter may not be waiting for flow control.
                return Ok(());
            }

            // Drain any newly unblocked consecutive frames.
            self.drain_resp_segmenter()?;

            return Ok(());
        }

        // Otherwise feed into request reassembler
        match self
            .req_reassembler
            .feed(data)
            .map_err(EcuNodeError::IsoTp)?
        {
            ReassembleResult::Complete { len } => {
                if let Some(uds_bytes) = self.req_reassembler.message(len) {
                    let mut buf: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                    buf.extend_from_slice(&uds_bytes[..len])
                        .map_err(|_| EcuNodeError::FrameTooLong)?;
                    self.req_reassembler.reset();

                    // Translate the arrival identifier into the UDS address space. The
                    // server compares logical addresses and must never need to know what
                    // a CAN identifier is - that is the layering this crate preserves.
                    let uds_dst = match arrival {
                        CanArrival::PhysicalRequest => u32::from(self.server.physical_address()),
                        CanArrival::FunctionalRequest => {
                            u32::from(self.server.functional_address())
                        }
                        CanArrival::FlowControlForOurResponse => {
                            debug_assert!(false, "flow control returns before reassembly");
                            return Ok(());
                        }
                    };

                    let uds_frame =
                        InboundFrame::new(frame.src.clone(), NodeAddress(uds_dst), &buf);
                    self.server
                        .handle(&uds_frame, now)
                        .map_err(EcuNodeError::Server)?;
                } else {
                    self.req_reassembler.reset();
                }
            }
            ReassembleResult::FlowControl { frame, len: fc_len } => {
                // Send FC back to gateway's request segmenter
                let mut fc: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                fc.extend_from_slice(&frame[..fc_len])
                    .map_err(|_| EcuNodeError::FrameTooLong)?;

                self.can_outbox
                    .push((NodeAddress(self.request_can_id), fc))
                    .map_err(|_| EcuNodeError::CanOutboxFull)?;
            }
            ReassembleResult::InProgress => {}
            ReassembleResult::SessionAborted {
                flow_control,
                fc_len,
            } => {
                let mut fc: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                fc.extend_from_slice(&flow_control[..fc_len])
                    .map_err(|_| EcuNodeError::CanOutboxFull)?;
                // Previously unguarded: an abort flow control frame was dropped with
                // no check and no signal, stalling the ISO-TP transfer.
                self.can_outbox
                    .push((NodeAddress(self.request_can_id), fc))
                    .map_err(|_| EcuNodeError::CanOutboxFull)?;

                self.req_reassembler.reset();
            }
        }

        Ok(())
    }

    // endregion: CAN frame input

    // region: Tick

    /// Advances server timers and drains server response outbox into
    /// the ISO-TP response segmenter.
    pub fn tick(&mut self, now: Instant) -> Result<(), EcuNodeError<H::Error>> {
        self.server.tick(now).map_err(EcuNodeError::Server)?;

        // Drain server outbox into response segmenter
        let mut srv_out: Vec<(NodeAddress, Vec<u8, UDS_MAX_FRAME>), UDS_MAX_OUTBOX> = Vec::new();
        self.server
            .drain_outbox(&mut srv_out)
            .map_err(|_| EcuNodeError::FrameTooLong)?;

        for (_, uds_data) in &srv_out {
            // Start segmenting the UDS response
            self.resp_segmenter
                .start(uds_data)
                .map_err(EcuNodeError::IsoTp)?;

            // Drain initial frames (SF or FF - subsequent CFs come after FC)
            self.drain_resp_segmenter()?;
        }

        Ok(())
    }

    // endregion: Tick

    // region: CAN outbox

    /// Drains outbound ISO-TP CAN frames.
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

    // endregion: CAN outbox

    // region: Internal helpers

    /// Drains the response segmenter into `can_outbox` until it either
    /// completes, runs out of frames, or needs a flow control frame.
    fn drain_resp_segmenter(&mut self) -> Result<(), EcuNodeError<H::Error>> {
        let mut out_buf = [0u8; CAN_MAX_FRAME];

        loop {
            match self
                .resp_segmenter
                .next_frame(&mut out_buf)
                .map_err(EcuNodeError::IsoTp)?
            {
                SegmentResult::Complete => break,
                // Waiting for FC from gateway resp_reassembler - stop here.
                // FC arrives via handle_can_frame next tick.
                SegmentResult::WaitForFlowControl => break,
                SegmentResult::Frame { len } => {
                    let mut frame: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                    frame
                        .extend_from_slice(&out_buf[..len])
                        .map_err(|_| EcuNodeError::FrameTooLong)?;

                    // Frames go to response_can_id - gateway listens there
                    self.can_outbox
                        .push((NodeAddress(self.response_can_id), frame))
                        .map_err(|_| EcuNodeError::CanOutboxFull)?;
                }
            }
        }

        Ok(())
    }

    // endregion: Internal helpers
}

// endregion: EcuNode
