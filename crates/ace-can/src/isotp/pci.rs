use crate::constants::{
    CF_TYPE, FC_TYPE, FF_ESCAPE, FF_MAX_LEN_CLASSIC, FF_TYPE, NIBBLE_MASK, SF_TYPE, TYPE_MASK,
};
use crate::error::IsoTpError;

// region: FlowStatus

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowStatus {
    /// Receiver is ready - sender may continue transmitting.
    ContinueToSend,
    /// Receiver is not ready - sender must wait for another FC.
    Wait,
    /// Receiver buffer overflow - sender must abort.
    Overflow,
}

impl TryFrom<u8> for FlowStatus {
    type Error = IsoTpError;

    fn try_from(nibble: u8) -> Result<Self, Self::Error> {
        match nibble & NIBBLE_MASK {
            0x0 => Ok(Self::ContinueToSend),
            0x1 => Ok(Self::Wait),
            0x2 => Ok(Self::Overflow),
            other => Err(IsoTpError::UnknownFlowStatus(other)),
        }
    }
}

// endregion: FlowStatus

// region: PciFrame

/// A decoded ISO-TP PCI frame.
///
/// Both `parse` and `encode_header` operate purely on PCI bytes - byte 0
/// is always the PCI byte. Address prefix bytes (N_TA for extended addressing,
/// N_AE for mixed addressing) are entirely the caller's responsibility:
///
/// - On receive: strip the address byte before calling `parse`
/// - On transmit: prepend the address byte after calling `encode_header`
///
/// This keeps both state machines free of addressing concerns and makes
/// the API contract explicit at the transport boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PciFrame<'a> {
    /// Complete message fits in a single CAN frame.
    SingleFrame {
        /// Total payload length (1-7 classic CAN normal, 1-6 extended/mixed,
        /// up to 62 CAN FD normal).
        len: u8,
        /// Payload bytes immediately following the PCI byte.
        data: &'a [u8],
    },
    /// Start of a multi-frame message.
    FirstFrame {
        /// Total message length across all frames.
        total_len: u32,
        /// Payload bytes carried in this first frame.
        data: &'a [u8],
    },
    /// Continuation of a multi-frame message.
    ConsecutiveFrame {
        /// Sequence number (1-F, then wraps to 0).
        sequence_number: u8,
        /// Payload bytes in this frame.
        data: &'a [u8],
    },
    /// Flow control sent by receiver to regulate sender.
    FlowControl {
        status: FlowStatus,
        /// Number of consecutive frames to send before waiting for next FC.
        /// 0 means send all remaining frames without waiting.
        block_size: u8,
        /// Minimum separation time between consecutive frames in milliseconds.
        /// Values 0xF1-0xF9 represent 100-900 microseconds.
        st_min: u8,
    },
}

impl<'a> PciFrame<'a> {
    /// Parses a `PciFrame` from a byte slice where byte 0 is the PCI byte.
    ///
    /// The caller must strip any addressing mode prefix byte (N_TA or N_AE)
    /// before calling this function.
    pub fn parse(pci_bytes: &'a [u8]) -> Result<Self, IsoTpError> {
        if pci_bytes.is_empty() {
            return Err(IsoTpError::FrameTooShort { actual: 0 });
        }

        let pci_byte = pci_bytes[0];
        let frame_type = pci_byte & TYPE_MASK;
        let lower_nibble = pci_byte & NIBBLE_MASK;

        match frame_type {
            // region: Single Frame
            x if x == SF_TYPE => {
                // CAN FD escape: SF lower nibble 0 uses second byte as length
                let (len, data) = if lower_nibble == 0 {
                    if pci_bytes.len() < 2 {
                        return Err(IsoTpError::FrameTooShort {
                            actual: pci_bytes.len(),
                        });
                    }
                    let len = pci_bytes[1];
                    (len, &pci_bytes[2..])
                } else {
                    (lower_nibble, &pci_bytes[1..])
                };

                if len == 0 {
                    return Err(IsoTpError::EmptySingleFrame);
                }

                Ok(PciFrame::SingleFrame { len, data })
            }
            // endregion: Single Frame

            // region: First Frame
            x if x == FF_TYPE => {
                if pci_bytes.len() < 2 {
                    return Err(IsoTpError::FrameTooShort {
                        actual: pci_bytes.len(),
                    });
                }

                let raw_len = ((lower_nibble as u16) << 8) | pci_bytes[1] as u16;

                // CAN FD escape: FF length 0x000 means read 4-byte length
                let (total_len, data) = if raw_len == FF_ESCAPE {
                    if pci_bytes.len() < 6 {
                        return Err(IsoTpError::FrameTooShort {
                            actual: pci_bytes.len(),
                        });
                    }
                    let len = u32::from_be_bytes([
                        pci_bytes[2],
                        pci_bytes[3],
                        pci_bytes[4],
                        pci_bytes[5],
                    ]);
                    (len, &pci_bytes[6..])
                } else {
                    (raw_len as u32, &pci_bytes[2..])
                };

                if total_len == 0 {
                    return Err(IsoTpError::InvalidLength);
                }

                Ok(PciFrame::FirstFrame { total_len, data })
            }
            // endregion: First Frame

            // region: Consecutive Frame
            x if x == CF_TYPE => {
                if pci_bytes.len() < 2 {
                    return Err(IsoTpError::FrameTooShort {
                        actual: pci_bytes.len(),
                    });
                }

                Ok(PciFrame::ConsecutiveFrame {
                    sequence_number: lower_nibble,
                    data: &pci_bytes[1..],
                })
            }
            // endregion: Consecutive Frame

            // region: Flow Control
            x if x == FC_TYPE => {
                if pci_bytes.len() < 3 {
                    return Err(IsoTpError::FrameTooShort {
                        actual: pci_bytes.len(),
                    });
                }

                let status = FlowStatus::try_from(lower_nibble)?;

                Ok(PciFrame::FlowControl {
                    status,
                    block_size: pci_bytes[1],
                    st_min: pci_bytes[2],
                })
            }
            // endregion: Flow Control
            other => Err(IsoTpError::UnknownFrameType(other >> 4)),
        }
    }

    /// Writes PCI header bytes into `buf` starting at byte 0.
    ///
    /// Does not write any addressing prefix byte - the caller prepends
    /// N_TA or N_AE for extended/mixed addressing after this call.
    /// Returns the number of PCI bytes written.
    pub fn encode_header(&self, buf: &mut [u8]) -> Result<usize, IsoTpError> {
        match self {
            PciFrame::SingleFrame { len, .. } => {
                let needed = if *len > 0x0F { 2 } else { 1 };
                if buf.len() < needed {
                    return Err(IsoTpError::FrameTooShort { actual: buf.len() });
                }
                if *len > 0x0F {
                    buf[0] = SF_TYPE;
                    buf[1] = *len;
                    Ok(2)
                } else {
                    buf[0] = SF_TYPE | (len & NIBBLE_MASK);
                    Ok(1)
                }
            }

            PciFrame::FirstFrame { total_len, .. } => {
                let needed = if *total_len > FF_MAX_LEN_CLASSIC {
                    6
                } else {
                    2
                };
                if buf.len() < needed {
                    return Err(IsoTpError::FrameTooShort { actual: buf.len() });
                }
                if *total_len > FF_MAX_LEN_CLASSIC {
                    buf[0] = FF_TYPE;
                    buf[1] = 0x00;
                    buf[2..6].copy_from_slice(&total_len.to_be_bytes());
                    Ok(6)
                } else {
                    let high = ((*total_len >> 8) as u8) & NIBBLE_MASK;
                    let low = (*total_len & 0xFF) as u8;
                    buf[0] = FF_TYPE | high;
                    buf[1] = low;
                    Ok(2)
                }
            }

            PciFrame::ConsecutiveFrame {
                sequence_number, ..
            } => {
                if buf.is_empty() {
                    return Err(IsoTpError::FrameTooShort { actual: 0 });
                }
                buf[0] = CF_TYPE | (sequence_number & NIBBLE_MASK);
                Ok(1)
            }

            PciFrame::FlowControl {
                status,
                block_size,
                st_min,
            } => {
                if buf.len() < 3 {
                    return Err(IsoTpError::FrameTooShort { actual: buf.len() });
                }
                let status_nibble = match status {
                    FlowStatus::ContinueToSend => 0x0,
                    FlowStatus::Wait => 0x1,
                    FlowStatus::Overflow => 0x2,
                };
                buf[0] = FC_TYPE | status_nibble;
                buf[1] = *block_size;
                buf[2] = *st_min;
                Ok(3)
            }
        }
    }
}

// endregion: PciFrame
