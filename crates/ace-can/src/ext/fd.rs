use crate::constants::{
    BRS_FLAG, CAN_FD_MAX_DLC, CAN_FD_MIN_LEN, DLC_OFFSET, EDL_FLAG, EFF_FLAG, ERR_FLAG, ESI_FLAG,
    EXT_ID_MASK, FLAGS_OFFSET, RRS_FLAG, STD_ID_MASK,
};
use crate::isotp::address::IsoTpAddressingMode;
use crate::{constants::DATA_OFFSET, error::CanError};
use ace_proto::can::frame::fd::{CanFdFrame, CanFdFrameMut};

// region: Offsets

/// Byte layout of a raw CAN FD frame:
///
/// [0..=3] - 32-bit CAN ID word (big-endian)
///           bit 31: extended frame flag (EFF)
///           bit 30: RRS (replaces RTR in CAN FD, must be 0)
///           bit 29: error frame flag (ERR)
///           bits 28-0: CAN ID
/// [4]     - flags byte
///           bit 2: EDL (Extended Data Length) - must be 1 for CAN FD
///           bit 1: BRS (Bit Rate Switch)
///           bit 0: ESI (Error State Indicator)
/// [5]     - DLC (0–15, maps to 0–64 bytes via dlc_to_len)
/// [6..]   - data bytes (0–64 bytes)

// endregion: Offsets

// region: DLC helpers

/// Maps a CAN FD DLC value (0–15) to the actual data length in bytes.
pub fn dlc_to_len(dlc: u8) -> usize {
    match dlc {
        0..=8 => dlc as usize,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        15 => 64,
        _ => 64,
    }
}

/// Maps a data length in bytes to the smallest valid CAN FD DLC.
pub fn len_to_dlc(len: usize) -> Option<u8> {
    match len {
        0..=8 => Some(len as u8),
        9..=12 => Some(9),
        13..=16 => Some(10),
        17..=20 => Some(11),
        21..=24 => Some(12),
        25..=32 => Some(13),
        33..=48 => Some(14),
        49..=64 => Some(15),
        _ => None,
    }
}

// endregion: DLC helpers

// region: CanFdFrameExt

pub trait CanFdFrameExt {
    fn as_bytes(&self) -> &[u8];

    // region: Raw field accessors

    fn id_word(&self) -> Option<u32> {
        let b = self.as_bytes();
        if b.len() < DATA_OFFSET {
            return None;
        }
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn flags_byte(&self) -> Option<u8> {
        self.as_bytes().get(FLAGS_OFFSET).copied()
    }

    fn dlc(&self) -> Option<u8> {
        self.as_bytes().get(DLC_OFFSET).copied()
    }

    fn data_bytes(&self) -> Option<&[u8]> {
        let b = self.as_bytes();
        let dlc = self.dlc()?;
        let len = dlc_to_len(dlc);
        let end = DATA_OFFSET + len;
        if b.len() < end {
            return None;
        }
        Some(&b[DATA_OFFSET..end])
    }

    // endregion: Raw field accessors

    // region: Flags

    fn is_extended_frame(&self) -> bool {
        self.id_word().map_or(false, |w| w & EFF_FLAG != 0)
    }

    fn is_rrs(&self) -> bool {
        self.id_word().map_or(false, |w| w & RRS_FLAG != 0)
    }

    fn is_error_frame(&self) -> bool {
        self.id_word().map_or(false, |w| w & ERR_FLAG != 0)
    }

    fn is_edl(&self) -> bool {
        self.flags_byte().map_or(false, |f| f & EDL_FLAG != 0)
    }

    fn is_brs(&self) -> bool {
        self.flags_byte().map_or(false, |f| f & BRS_FLAG != 0)
    }

    fn is_esi(&self) -> bool {
        self.flags_byte().map_or(false, |f| f & ESI_FLAG != 0)
    }

    // endregion: Flags

    // region: Typed accessors

    fn can_id(&self) -> Option<Result<ace_proto::can::id::CanId, CanError>> {
        let word = self.id_word()?;
        if word & EFF_FLAG != 0 {
            let raw = word & EXT_ID_MASK;
            Some(
                ace_proto::can::id::ExtendedCanId::new(raw)
                    .map(ace_proto::can::id::CanId::Extended)
                    .map_err(CanError::from),
            )
        } else {
            let raw = (word & STD_ID_MASK) as u16;
            Some(
                ace_proto::can::id::StandardCanId::new(raw)
                    .map(ace_proto::can::id::CanId::Standard)
                    .map_err(CanError::from),
            )
        }
    }

    // endregion: Typed accessors

    // region: Validation

    fn validate(&self) -> Result<(), CanError> {
        let b = self.as_bytes();

        if b.len() < CAN_FD_MIN_LEN {
            return Err(CanError::BufferTooShort {
                expected: CAN_FD_MIN_LEN,
                actual: b.len(),
            });
        }

        let flags = b[FLAGS_OFFSET];

        // EDL must be set on all CAN FD frames
        if flags & EDL_FLAG == 0 {
            return Err(CanError::InvalidFlags);
        }

        // BRS requires EDL - already guaranteed above, but ESI without EDL is illegal
        if flags & ESI_FLAG != 0 && flags & EDL_FLAG == 0 {
            return Err(CanError::InvalidFlags);
        }

        // RRS must be 0 on CAN FD frames
        let word = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        if word & RRS_FLAG != 0 {
            return Err(CanError::InvalidFlags);
        }

        let dlc = b[DLC_OFFSET];
        if dlc > CAN_FD_MAX_DLC {
            return Err(CanError::InvalidDlc(dlc));
        }

        let expected_len = DATA_OFFSET + dlc_to_len(dlc);
        if b.len() < expected_len {
            return Err(CanError::BufferTooShort {
                expected: expected_len,
                actual: b.len(),
            });
        }

        Ok(())
    }

    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    // endregion: Validation

    // region: ISO-TP helpers

    fn isotp_bytes(&self, mode: &IsoTpAddressingMode) -> Option<&[u8]> {
        let data = self.data_bytes()?;
        let offset = mode.pci_offset();
        if data.len() <= offset {
            return None;
        }
        Some(&data[offset..])
    }

    // endregion: ISO-TP helpers
}

impl<'a> CanFdFrameExt for CanFdFrame<'a> {
    fn as_bytes(&self) -> &[u8] {
        ace_proto::common::RawFrame::as_bytes(self)
    }
}

impl<'a> CanFdFrameExt for CanFdFrameMut<'a> {
    fn as_bytes(&self) -> &[u8] {
        ace_proto::common::RawFrame::as_bytes(self)
    }
}

// endregion: CanFdFrameExt

// region: CanFdFrameMutExt

pub trait CanFdFrameMutExt: CanFdFrameExt {
    fn as_bytes_mut(&mut self) -> &mut [u8];

    fn set_id_word(&mut self, word: u32) -> Result<(), CanError> {
        let b = self.as_bytes_mut();
        if b.len() < DATA_OFFSET {
            return Err(CanError::BufferTooShort {
                expected: DATA_OFFSET,
                actual: b.len(),
            });
        }
        b[0..4].copy_from_slice(&word.to_be_bytes());
        Ok(())
    }

    fn set_standard_id(&mut self, id: &ace_proto::can::id::StandardCanId) -> Result<(), CanError> {
        let word = id.value() as u32 & STD_ID_MASK;
        self.set_id_word(word)
    }

    fn set_extended_id(&mut self, id: &ace_proto::can::id::ExtendedCanId) -> Result<(), CanError> {
        let word = (id.value() & EXT_ID_MASK) | EFF_FLAG;
        self.set_id_word(word)
    }

    fn set_flags(&mut self, edl: bool, brs: bool, esi: bool) -> Result<(), CanError> {
        let b = self.as_bytes_mut();
        if b.len() <= FLAGS_OFFSET {
            return Err(CanError::BufferTooShort {
                expected: FLAGS_OFFSET + 1,
                actual: b.len(),
            });
        }
        let mut flags = EDL_FLAG; // EDL always set for CAN FD
        if brs {
            flags |= BRS_FLAG;
        }
        if esi {
            flags |= ESI_FLAG;
        }
        if !edl {
            return Err(CanError::InvalidFlags);
        }
        b[FLAGS_OFFSET] = flags;
        Ok(())
    }

    fn set_dlc(&mut self, dlc: u8) -> Result<(), CanError> {
        if dlc > CAN_FD_MAX_DLC {
            return Err(CanError::InvalidDlc(dlc));
        }
        let b = self.as_bytes_mut();
        if b.len() <= DLC_OFFSET {
            return Err(CanError::BufferTooShort {
                expected: DLC_OFFSET + 1,
                actual: b.len(),
            });
        }
        b[DLC_OFFSET] = dlc;
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), CanError> {
        let dlc = len_to_dlc(data.len()).ok_or(CanError::InvalidDlc(data.len() as u8))?;
        let b = self.as_bytes_mut();
        let end = DATA_OFFSET + data.len();
        if b.len() < end {
            return Err(CanError::BufferTooShort {
                expected: end,
                actual: b.len(),
            });
        }
        b[DATA_OFFSET..DATA_OFFSET + data.len()].copy_from_slice(data);
        b[DLC_OFFSET] = dlc;
        Ok(())
    }
}

impl<'a> CanFdFrameMutExt for CanFdFrameMut<'a> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        ace_proto::common::RawFrameMut::as_bytes_mut(self)
    }
}

// endregion: CanFdFrameMutExt
