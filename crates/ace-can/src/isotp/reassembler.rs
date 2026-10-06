use crate::constants::NIBBLE_MASK;
use crate::error::IsoTpError;
use crate::isotp::address::IsoTpAddressingMode;
use crate::isotp::pci::{FlowStatus, PciFrame};

// region: ReassemblerConfig

/// Configuration for an ISO-TP reassembler session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReassemblerConfig {
    pub addressing_mode: IsoTpAddressingMode,
    /// Block size to advertise in flow control frames.
    /// 0 = send all consecutive frames without waiting.
    pub block_size: u8,
    /// Minimum separation time to advertise in flow control frames (ms).
    pub st_min: u8,
}

impl ReassemblerConfig {
    pub fn new(addressing_mode: IsoTpAddressingMode) -> Self {
        Self {
            addressing_mode,
            block_size: 0,
            st_min: 0,
        }
    }
}

// endregion: ReassemblerConfig

// region: ReassemblerState

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReassemblerState {
    Idle,
    Active {
        total_len: u32,
        received: usize,
        next_sequence: u8,

        /// Consecutive frames still permitted before the sender must wait for another
        /// flow control.
        ///
        /// Counted down from the configured block size. Without this the reassembler
        /// advertised a block size in the flow control it sent and then never sent
        /// another, so any sender honouring that block size waited forever.
        block_remaining: u8,
    },
}

// endregion: ReassemblerState

// region: ReassembleResult

/// Outcome of feeding PCI bytes to the reassembler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReassembleResult {
    /// Frame consumed, message not yet complete.
    InProgress,
    /// Message is complete - `len` bytes are valid in the buffer.
    Complete { len: usize },
    /// First frame received - caller must send this FC on the wire.
    ///
    /// Contains raw PCI bytes only (3 bytes, no address prefix).
    /// For extended/mixed addressing the caller prepends N_TA or N_AE
    /// before putting the frame on the wire.
    FlowControl { frame: [u8; 3], len: usize },
    /// New first frame arrived mid-session - previous session abandoned.
    /// Caller must send this FC on the wire.
    SessionAborted {
        flow_control: [u8; 3],
        fc_len: usize,
    },
}

// endregion: ReassembleResult

// region: Reassembler

/// Stateful ISO-TP reassembler.
///
/// Operates on raw PCI bytes - the caller strips the addressing prefix byte
/// (N_TA for extended, N_AE for mixed) before calling `feed`. Flow control
/// frames returned contain raw PCI bytes only - the caller prepends the
/// address prefix byte before putting them on the wire.
///
/// Buffer size `N` must be large enough to hold the complete reassembled
/// message. For classic CAN ISO-TP the maximum is 4095 bytes.
pub struct Reassembler<const MAX_FRAME: usize> {
    config: ReassemblerConfig,
    buf: [u8; MAX_FRAME],
    state: ReassemblerState,
}

impl<const MAX_FRAME: usize> Reassembler<MAX_FRAME> {
    pub fn new(config: ReassemblerConfig) -> Self {
        Self {
            config,
            buf: [0u8; MAX_FRAME],
            state: ReassemblerState::Idle,
        }
    }

    /// Resets to idle, discarding any in-progress session.
    /// Builds a continue-to-send flow control carrying this reassembler's block size
    /// and separation time.
    ///
    /// Shared by the first frame and every block boundary, so the block size a receiver
    /// advertises cannot drift from the one it enforces.
    fn continue_to_send(&self) -> Result<[u8; 3], IsoTpError> {
        let fc_pci = PciFrame::FlowControl {
            status: FlowStatus::ContinueToSend,
            block_size: self.config.block_size,
            st_min: self.config.st_min,
        };

        let mut fc_buf = [0u8; 3];
        fc_pci.encode_header(&mut fc_buf)?;

        Ok(fc_buf)
    }

    pub fn reset(&mut self) {
        self.state = ReassemblerState::Idle;
    }

    /// Returns the reassembled message bytes after a `Complete` result.
    /// Returns `None` if reassembly is still in progress.
    pub fn message(&self, len: usize) -> Option<&[u8]> {
        match &self.state {
            ReassemblerState::Idle => Some(&self.buf[..len]),
            ReassemblerState::Active { .. } => None,
        }
    }

    /// Feeds raw PCI bytes into the reassembler.
    ///
    /// The caller must strip the addressing prefix byte before calling this
    /// function - byte 0 of `pci_bytes` must be the PCI byte.
    pub fn feed(&mut self, pci_bytes: &[u8]) -> Result<ReassembleResult, IsoTpError> {
        let pci = PciFrame::parse(pci_bytes)?;

        match pci {
            // region: Single Frame
            PciFrame::SingleFrame { len, data } => {
                let len = len as usize;
                if len == 0 {
                    return Err(IsoTpError::EmptySingleFrame);
                }
                if len > MAX_FRAME {
                    return Err(IsoTpError::PayloadTooLarge);
                }
                let copy_len = data.len().min(len).min(MAX_FRAME);
                self.buf[..copy_len].copy_from_slice(&data[..copy_len]);
                self.state = ReassemblerState::Idle;
                Ok(ReassembleResult::Complete { len })
            }
            // endregion: Single Frame

            // region: First Frame
            PciFrame::FirstFrame { total_len, data } => {
                let total = total_len as usize;
                if total == 0 {
                    return Err(IsoTpError::InvalidLength);
                }
                if total > MAX_FRAME {
                    return Err(IsoTpError::PayloadTooLarge);
                }

                let was_active = matches!(self.state, ReassemblerState::Active { .. });

                let copy_len = data.len().min(total).min(MAX_FRAME);
                self.buf[..copy_len].copy_from_slice(&data[..copy_len]);

                self.state = ReassemblerState::Active {
                    total_len,
                    received: copy_len,
                    next_sequence: 1,
                    block_remaining: self.config.block_size,
                };

                let fc_buf = self.continue_to_send()?;

                if was_active {
                    Ok(ReassembleResult::SessionAborted {
                        flow_control: fc_buf,
                        fc_len: 3,
                    })
                } else {
                    Ok(ReassembleResult::FlowControl {
                        frame: fc_buf,
                        len: 3,
                    })
                }
            }
            // endregion: First Frame

            // region: Consecutive Frame
            PciFrame::ConsecutiveFrame {
                sequence_number,
                data,
            } => {
                let (total_len, received, next_sequence, block_remaining) = match &self.state {
                    ReassemblerState::Active {
                        total_len,
                        received,
                        next_sequence,
                        block_remaining,
                    } => (*total_len, *received, *next_sequence, *block_remaining),
                    ReassemblerState::Idle => return Err(IsoTpError::UnexpectedConsecutiveFrame),
                };

                if sequence_number != next_sequence {
                    self.reset();
                    return Err(IsoTpError::SequenceError {
                        expected: next_sequence,
                        actual: sequence_number,
                    });
                }

                let total = total_len as usize;
                let remaining = total - received;
                let copy_len = data.len().min(remaining);

                self.buf[received..received + copy_len].copy_from_slice(&data[..copy_len]);

                let new_received = received + copy_len;
                let new_sequence = (next_sequence + 1) & NIBBLE_MASK;

                if new_received >= total {
                    // Completion wins over a block boundary: there is nothing left to
                    // authorise, so no further flow control is due.
                    self.state = ReassemblerState::Idle;
                    return Ok(ReassembleResult::Complete { len: total });
                }

                // A block size of zero means "send everything without further flow
                // control", so the counter is only meaningful above zero.
                let block_exhausted = self.config.block_size > 0 && block_remaining <= 1;

                self.state = ReassemblerState::Active {
                    total_len,
                    received: new_received,
                    next_sequence: new_sequence,
                    block_remaining: if block_exhausted {
                        self.config.block_size
                    } else {
                        block_remaining.saturating_sub(1)
                    },
                };

                if block_exhausted {
                    // The block the last flow control authorised is used up. ISO 15765-2
                    // requires a new flow control here; the sender is blocked until it
                    // arrives.
                    return Ok(ReassembleResult::FlowControl {
                        frame: self.continue_to_send()?,
                        len: 3,
                    });
                }

                Ok(ReassembleResult::InProgress)
            }
            // endregion: Consecutive Frame

            // FC received by reassembler is unexpected
            PciFrame::FlowControl { .. } => Err(IsoTpError::UnknownFrameType(0x3)),
        }
    }
}

// endregion: Reassembler
