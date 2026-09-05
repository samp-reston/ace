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
use ace_core::{clock::Instant, io::NodeAddress};
use ace_server::{
    NrcError,
    handler::ServerHandler,
    security_provider::SecurityProvider,
    server::{ServerError, UdsServer},
};

// region: EcuNodeError

#[derive(Debug)]
pub enum EcuNodeError<SE: NrcError> {
    IsoTp(ace_can::IsoTpError),
    CanOutboxFull,
    Server(ServerError<SE>),
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
    pub fn new(
        logical_address: u16,
        request_can_id: u32,
        response_can_id: u32,
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
    pub fn handle_can_frame(
        &mut self,
        data: &[u8],
        now: Instant,
    ) -> Result<(), EcuNodeError<H::Error>> {
        // Check if this is a flow control frame for our response segmenter
        // FC frames have upper nibble 0x3 - if we are waiting for FC, feed it
        if let Some(&first) = data.first() {
            if first & 0xF0 == 0x30 {
                // Flow control for our response segmenter
                if let Err(e) = self.resp_segmenter.handle_flow_control(data) {
                    // Non-fatal - segmenter may not be waiting for FC
                    let _ = e;
                } else {
                    // Drain any newly unblocked consecutive frames
                    self.drain_resp_segmenter()?;
                }

                return Ok(());
            }
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
                    let _ = buf.extend_from_slice(&uds_bytes[..len.min(CAN_MAX_FRAME)]);
                    self.req_reassembler.reset();

                    // Feed UDS request to server
                    self.server
                        .handle(&NodeAddress(self.request_can_id), &buf, now)
                        .map_err(EcuNodeError::Server)?;
                } else {
                    self.req_reassembler.reset();
                }
            }
            ReassembleResult::FlowControl { frame, len: fc_len } => {
                // Send FC back to gateway's request segmenter
                let mut fc: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                let _ = fc.extend_from_slice(&frame[..fc_len]);

                if self.can_outbox.len() >= CAN_MAX_OUTBOX {
                    return Err(EcuNodeError::CanOutboxFull);
                } else {
                    self.can_outbox.push((NodeAddress(self.request_can_id), fc));
                }
            }
            ReassembleResult::InProgress => {}
            ReassembleResult::SessionAborted {
                flow_control,
                fc_len,
            } => {
                let mut fc: Vec<u8, CAN_MAX_FRAME> = Vec::new();
                let _ = fc.extend_from_slice(&flow_control[..fc_len]);
                let _ = self.can_outbox.push((NodeAddress(self.request_can_id), fc));

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
        self.server.drain_outbox(&mut srv_out);

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
    ) -> usize {
        let n = self.can_outbox.len();

        for item in self.can_outbox.drain(..) {
            let _ = out.push(item);
        }

        n
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
                    let _ = frame.extend_from_slice(&out_buf[..len]);

                    // Frames go to response_can_id - gateway listens there
                    if self.can_outbox.len() >= CAN_MAX_OUTBOX {
                        return Err(EcuNodeError::CanOutboxFull);
                    } else {
                        self.can_outbox
                            .push((NodeAddress(self.response_can_id), frame));
                    }
                }
            }
        }

        Ok(())
    }

    // endregion: Internal helpers
}

// endregion: EcuNode
