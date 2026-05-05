//! IsoTpNode - a SimNode on the CAN bus that bridges raw UDS bytes (from the gateway) to ISO-TP
//! segmented CAN frames (to the ECU) and back.
//!
//! The gateway addresses IsoTpNode by CAN request ID. IsoTpNode addresses the ECU UdsServer by its
//! NodeAddress. CAN frames addressed to the response CAN ID are reassembled and forwarded to the
//! UdsServer, whose response is segmented and put back onto the CAN bus.

// region: Imports

use ace_can::{
    IsoTpAddressingMode, IsoTpError, ReassembleResult, Reassembler, ReassemblerConfig,
    SegmentResult, Segmenter, SegmenterConfig,
};
use ace_sim::{clock::Instant, io::NodeAddress};

// endregion: Imports

// region: Capacity Constants

/// Max reassembled UDS message size.
pub const ISOTP_MAX_UDS: usize = 4096;

/// Max CAN frame payload bytes (classic CAN = 8, CAN FD = 64).
pub const ISOTP_MAX_FRAME: usize = 64;

/// Max outbox depth.
pub const ISOTP_MAX_OUT: usize = 64;

// endregion: Capacity Constants

// region: IsoTpError

#[derive(Debug)]
pub enum IsoTpNodeError {
    Reassembler(IsoTpError),
    Segmenter(IsoTpError),
    OutboxFull,
}

// endregion: IsoTpError

// region: IsoTpNode

/// A SimNode that translates between raw UDS bytes and ISO-TP CAN frames.
///
/// Sits on the CAN simul.ation bus between the DoIP gateway and the UdsServer.
///
/// **Receive Path** (gateway -> ECU):
///     Gateway sends raw UDS bytes addressed to `request_can_id`. IsoTpNode segments them into CAN
///     frames and puts them on the bus addressed to the ECU's NodeAddress.
///
/// **Transmit path** (ECU -> gateway):
///     ECU UdsServer puts raw UDS bytes in its outbox addressed to IsoTpNode. IsoTpNode segments
///     and puts CAN frames on the bus addressed to `response_can_id` so the gateway can route them
///     back to the tester.
pub struct IsoTpNode<const N: usize = ISOTP_MAX_UDS> {
    /// CAN ID this node uses for receiving requests from the gateway.
    request_can_id: u32,

    /// CAN ID this node uses for sending responses to the gateway.
    response_can_id: u32,

    /// This node's own NodeAddress on the CAN sim bus.
    address: NodeAddress,
    reassembler: Reassembler<N>,
    req_segmenter: Segmenter<N>,
    resp_segmenter: Segmenter<N>,

    /// Outbound CAN frames for the CAN bus.
    pub can_outbox: heapless::Vec<(NodeAddress, heapless::Vec<u8, ISOTP_MAX_FRAME>), ISOTP_MAX_OUT>,

    /// Outbound UDS bytes for the UdsServer.
    uds_outbox: heapless::Vec<(NodeAddress, heapless::Vec<u8, N>), 4>,
}

impl<const N: usize> IsoTpNode<N> {
    pub fn new(
        request_can_id: u32,
        response_can_id: u32,
        addressing_mode: IsoTpAddressingMode,
    ) -> Self {
        let address = NodeAddress(response_can_id);

        let rsm_config = ReassemblerConfig::new(addressing_mode.clone());
        let seg_config = SegmenterConfig::classic(addressing_mode);

        Self {
            request_can_id,
            response_can_id,
            address,
            reassembler: Reassembler::new(rsm_config),
            req_segmenter: Segmenter::new(seg_config.clone()),
            resp_segmenter: Segmenter::new(seg_config),
            can_outbox: heapless::Vec::new(),
            uds_outbox: heapless::Vec::new(),
        }
    }

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }

    // region: Gateway -> ECU Path

    /// Receives raw UDS bytes from the gateway and segments them into CAN frames addressed to the
    /// ECU.
    pub fn handle_from_gateway(
        &mut self,
        uds_data: &[u8],
        _now: Instant,
    ) -> Result<(), IsoTpNodeError> {
        self.req_segmenter
            .start(uds_data)
            .map_err(IsoTpNodeError::Segmenter)?;

        let ecu_rx_addr = NodeAddress(self.response_can_id);
        Self::drain_segmenter(&mut self.req_segmenter, ecu_rx_addr, &mut self.can_outbox)
    }

    // endregion: Gateway -> ECU Path

    // region: ECU -> Gateway Path

    /// Segments raw UDS response bytes from the UdsServer outbox into CAN frames addressed to
    /// `request_can_id` (the gateway's receive CAN ID).
    ///
    /// This is the correct entry point for UdsServer outbox data - NOT `handle_from_ecu` which
    /// expectes reassebled CAN frames, not UDS bytes.
    pub fn handle_uds_response(
        &mut self,
        uds_data: &[u8],
        _now: Instant,
    ) -> Result<(), IsoTpNodeError> {
        self.resp_segmenter
            .start(uds_data)
            .map_err(IsoTpNodeError::Segmenter)?;

        // Response CAN frames are address to gateway's receive ID
        let gw_rx_addr = NodeAddress(self.request_can_id);
        Self::drain_segmenter(&mut self.resp_segmenter, gw_rx_addr, &mut self.can_outbox)
    }

    /// Receives a CAN frame fro mthe ECU (via the reassembler) and produces reassembled UDS bytes
    /// for the gateway.
    pub fn handle_from_ecu(
        &mut self,
        can_frame: &[u8],
        _now: Instant,
    ) -> Result<(), IsoTpNodeError> {
        match self
            .reassembler
            .feed(can_frame)
            .map_err(IsoTpNodeError::Reassembler)?
        {
            ReassembleResult::Complete { len } => {
                if let Some(uds_bytes) = self.reassembler.message(len) {
                    let gateway_addr = NodeAddress(self.request_can_id);
                    let mut frame = heapless::Vec::new();

                    let _ = frame.extend_from_slice(&uds_bytes[..len.min(N)]);
                    self.uds_outbox
                        .push((gateway_addr, frame))
                        .map_err(|_| IsoTpNodeError::OutboxFull)?;
                }
            }
            ReassembleResult::FlowControl { frame, len: fc_len } => {
                let ecu_addr = NodeAddress(self.response_can_id);
                let mut fc_frame = heapless::Vec::new();

                let _ = fc_frame.extend_from_slice(&frame[..fc_len]);
                self.can_outbox
                    .push((ecu_addr, fc_frame))
                    .map_err(|_| IsoTpNodeError::OutboxFull)?;
            }
            ReassembleResult::InProgress => {}
            ReassembleResult::SessionAborted {
                flow_control,
                fc_len,
            } => {
                let ecu_addr = NodeAddress(self.response_can_id);
                let mut fc_frame = heapless::Vec::new();

                let _ = fc_frame.extend_from_slice(&flow_control[..fc_len]);
                let _ = self.can_outbox.push((ecu_addr, fc_frame));

                self.reassembler.reset();
            }
        }

        Ok(())
    }

    // endregion: ECU -> Gateway Path

    // region: Outbox Drains

    /// Drains CAN frames destined for the CAN bus.
    pub fn drain_can_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, ISOTP_MAX_FRAME>), ISOTP_MAX_OUT>,
    ) -> usize {
        let n = self.can_outbox.len();

        for item in self.can_outbox.drain(..) {
            let _ = out.push(item);
        }

        n
    }

    /// Drains reassembled UDS bytes destined for the UdsServer.
    pub fn drain_uds_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, N>), 4>,
    ) -> usize {
        let n = self.uds_outbox.len();

        for item in self.uds_outbox.drain(..) {
            let _ = out.push(item);
        }

        n
    }

    // endregion: Outbox Drains

    // region: Internal helpers

    fn drain_segmenter(
        segmenter: &mut Segmenter<N>,
        dst: NodeAddress,
        can_outbox: &mut heapless::Vec<
            (NodeAddress, heapless::Vec<u8, ISOTP_MAX_FRAME>),
            ISOTP_MAX_OUT,
        >,
    ) -> Result<(), IsoTpNodeError> {
        let mut out_buf = [0u8; ISOTP_MAX_FRAME];
        loop {
            match segmenter
                .next_frame(&mut out_buf)
                .map_err(IsoTpNodeError::Segmenter)?
            {
                SegmentResult::Complete => break,
                SegmentResult::WaitForFlowControl => break,
                SegmentResult::Frame { len } => {
                    let mut frame: heapless::Vec<u8, ISOTP_MAX_FRAME> = heapless::Vec::new();
                    let _ = frame.extend_from_slice(&out_buf[..len]);
                    can_outbox
                        .push((dst.clone(), frame))
                        .map_err(|_| IsoTpNodeError::OutboxFull)?;
                }
            }
        }

        Ok(())
    }

    // endregion: Internal helpers
}

// endregion: IsoTpNode
