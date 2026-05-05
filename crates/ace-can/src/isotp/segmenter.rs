use crate::error::IsoTpError;
use crate::isotp::address::IsoTpAddressingMode;
use crate::isotp::pci::{FlowStatus, PciFrame};

// region: SegmenterConfig

/// Configuration for an ISO-TP segmenter session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmenterConfig {
    pub addressing_mode: IsoTpAddressingMode,

    /// Maximum data bytes per CAN frame - 8 for classic CAN, up to 64 for CAN FD.
    pub max_frame_data_len: usize,
}

impl SegmenterConfig {
    pub fn classic(addressing_mode: IsoTpAddressingMode) -> Self {
        Self {
            addressing_mode,
            max_frame_data_len: 8,
        }
    }

    pub fn fd(addressing_mode: IsoTpAddressingMode) -> Self {
        Self {
            addressing_mode,
            max_frame_data_len: 64,
        }
    }
}

// endregion: SegmenterConfig

// region: SegmenterState

#[derive(Debug, Clone, PartialEq, Eq)]
enum SegmenterState {
    Idle,

    WaitingForFlowControl {
        next_sequence: u8,
        sent: usize,
    },

    Sending {
        next_sequence: u8,
        sent: usize,
        /// 0 = send all remaining without waiting.
        block_remaining: u8,
        st_min: u8,
    },
}

// endregion: SegmenterState

// region: SegmentResult

/// Outcome of calling [`Segmenter::next_frame`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentResult {
    /// Raw PCI bytes written into `out_buf` - `len` bytes are valid.
    ///
    /// For extended/mixed addressing the caller prepends N_TA or N_AE
    /// before putting the frame on the wire.
    Frame { len: usize },

    /// All frames sent - session complete.
    Complete,

    /// Waiting for flow control - call [`Segmenter::handle_flow_control`]
    /// before calling [`Segmenter::next_frame`] again.
    WaitForFlowControl,
}

// endregion: SegmentResult

// region: Segmenter

/// Stateful ISO-TP segmenter.
///
/// Produces raw PCI bytes - the caller prepends the addressing prefix byte
/// (N_TA for extended, N_AE for mixed) before putting each frame on the wire.
///
/// `handle_flow_control` similarly expects raw PCI bytes - the caller strips
/// the addressing prefix byte before passing to this function.
///
/// `st_min` from received FC frames is stored but not enforced - the caller
/// is responsible for honouring the inter-frame delay on bare metal targets.
pub struct Segmenter<const N: usize = 4096> {
    config: SegmenterConfig,
    payload: heapless::Vec<u8, N>,
    state: SegmenterState,
}

impl<const N: usize> Segmenter<N> {
    pub fn new(config: SegmenterConfig) -> Self {
        Self {
            config,
            payload: heapless::Vec::new(),
            state: SegmenterState::Idle,
        }
    }

    /// Begins a new segmentation session with the given payload.
    pub fn start(&mut self, payload: &[u8]) -> Result<(), IsoTpError> {
        if payload.is_empty() {
            return Err(IsoTpError::InvalidLength);
        }

        if payload.len() > N {
            return Err(IsoTpError::PayloadTooLarge);
        }

        if payload.len() > u32::MAX as usize {
            return Err(IsoTpError::PayloadTooLarge);
        }

        self.payload.clear();
        self.payload
            .extend_from_slice(payload)
            .map_err(|_| IsoTpError::PayloadTooLarge)?;

        self.state = SegmenterState::Sending {
            next_sequence: 1,
            sent: 0,
            block_remaining: 0,
            st_min: 0,
        };

        Ok(())
    }

    /// Resets to idle, discarding any active session.
    pub fn reset(&mut self) {
        self.payload.clear();
        self.state = SegmenterState::Idle;
    }

    /// Handles an incoming flow control frame from the receiver.
    ///
    /// `pci_bytes` must be raw PCI bytes - the caller strips the addressing
    /// prefix byte before passing to this function.
    pub fn handle_flow_control(&mut self, pci_bytes: &[u8]) -> Result<(), IsoTpError> {
        let pci = PciFrame::parse(pci_bytes)?;

        let (next_sequence, sent) = match &self.state {
            SegmenterState::WaitingForFlowControl {
                next_sequence,
                sent,
            } => (*next_sequence, *sent),
            _ => return Err(IsoTpError::UnknownFrameType(0x3)),
        };

        match pci {
            PciFrame::FlowControl {
                status,
                block_size,
                st_min,
            } => match status {
                FlowStatus::ContinueToSend => {
                    self.state = SegmenterState::Sending {
                        next_sequence,
                        sent,
                        block_remaining: block_size,
                        st_min,
                    };
                    Ok(())
                }
                FlowStatus::Wait => Ok(()),
                FlowStatus::Overflow => {
                    self.reset();
                    Err(IsoTpError::PayloadTooLarge)
                }
            },
            _ => Err(IsoTpError::UnknownFrameType(0x0)),
        }
    }

    /// Writes the next ISO-TP frame as raw PCI bytes into `out_buf`.
    ///
    /// For extended/mixed addressing the caller prepends N_TA or N_AE
    /// before putting the frame on the wire.
    pub fn next_frame(&mut self, out_buf: &mut [u8]) -> Result<SegmentResult, IsoTpError> {
        match self.state.clone() {
            SegmenterState::Idle => Ok(SegmentResult::Complete),

            SegmenterState::WaitingForFlowControl { .. } => Ok(SegmentResult::WaitForFlowControl),

            SegmenterState::Sending {
                next_sequence,
                sent,
                block_remaining,
                st_min,
            } => {
                let total = self.payload.len();
                let max_data = self.config.max_frame_data_len;

                // Max single frame payload depends on frame type and addressing mode
                let max_sf = match max_data {
                    8 => self.config.addressing_mode.max_sf_payload_classic(),
                    _ => self.config.addressing_mode.max_sf_payload_fd(),
                };

                // region: Single frame

                if sent == 0 && total <= max_sf {
                    let pci = PciFrame::SingleFrame {
                        len: total as u8,
                        data: &self.payload,
                    };

                    let pci_len = pci.encode_header(out_buf)?;
                    let end = pci_len + total;

                    if out_buf.len() < end {
                        return Err(IsoTpError::FrameTooShort {
                            actual: out_buf.len(),
                        });
                    }
                    out_buf[pci_len..end].copy_from_slice(&self.payload);

                    self.state = SegmenterState::Idle;

                    return Ok(SegmentResult::Frame { len: end });
                }

                // endregion: Single frame

                // region: First frame

                if sent == 0 {
                    let pci = PciFrame::FirstFrame {
                        total_len: total as u32,
                        data: &self.payload,
                    };

                    let pci_len = pci.encode_header(out_buf)?;
                    let data_in_ff = (max_data - pci_len).min(total);
                    let end = pci_len + data_in_ff;

                    if out_buf.len() < end {
                        return Err(IsoTpError::FrameTooShort {
                            actual: out_buf.len(),
                        });
                    }
                    out_buf[pci_len..end].copy_from_slice(&self.payload[..data_in_ff]);

                    self.state = SegmenterState::WaitingForFlowControl {
                        next_sequence: 1,
                        sent: data_in_ff,
                    };

                    return Ok(SegmentResult::Frame { len: end });
                }

                // endregion: First frame

                // region: Consecutive frame

                let remaining = &self.payload[sent..];
                let pci = PciFrame::ConsecutiveFrame {
                    sequence_number: next_sequence,
                    data: remaining,
                };

                let pci_len = pci.encode_header(out_buf)?;
                let data_in_cf = (max_data - pci_len).min(remaining.len());
                let end = pci_len + data_in_cf;

                if out_buf.len() < end {
                    return Err(IsoTpError::FrameTooShort {
                        actual: out_buf.len(),
                    });
                }

                out_buf[pci_len..end].copy_from_slice(&remaining[..data_in_cf]);

                let new_sent = sent + data_in_cf;
                let new_sequence = (next_sequence + 1) & 0x0F;

                if new_sent >= total {
                    self.state = SegmenterState::Idle;
                    return Ok(SegmentResult::Frame { len: end });
                }

                let new_block = if block_remaining == 0 {
                    0
                } else if block_remaining == 1 {
                    self.state = SegmenterState::WaitingForFlowControl {
                        next_sequence: new_sequence,
                        sent: new_sent,
                    };
                    return Ok(SegmentResult::Frame { len: end });
                } else {
                    block_remaining - 1
                };

                self.state = SegmenterState::Sending {
                    next_sequence: new_sequence,
                    sent: new_sent,
                    block_remaining: new_block,
                    st_min,
                };

                Ok(SegmentResult::Frame { len: end })

                // endregion: Consecutive frame
            }
        }
    }
}

// endregion: Segmenter
