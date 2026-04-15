use crate::{
    constants::{
        CLASSIC_CAN_MAX_DLC, CLASSIC_CAN_MIN_LEN, DATA_OFFSET, DLC_OFFSET, EFF_FLAG, ERR_FLAG,
        EXT_ID_MASK, RTR_FLAG, STD_ID_MASK,
    },
    error::CanError,
    isotp::address::IsoTpAddressingMode,
};
use ace_core::AddressMode;
use ace_proto::can::frame::classic::{CanFrame, CanFrameMut};

// region: Offsets

/// Byte layout of a raw classic CAN frame:
///
/// [0..=3] - 32-bit CAN ID word (big-endian)
///           bit 31: extended frame flag (EFF)
///           bit 30: remote transmission request (RTR)
///           bit 29: error frame flag (ERR)
///           bits 28-0: CAN ID (11-bit in bits 10-0 for standard,
///                               29-bit in bits 28-0 for extended)
/// [4]     - DLC (0–8)
/// [5..=12]- data bytes (0–8 bytes, DLC determines count)

// endregion: Offsets

// region: CanFrameExt

pub trait CanFrameExt {
    fn as_bytes(&self) -> &[u8];

    // region: Raw field accessors

    fn id_word(&self) -> Option<u32> {
        let b = self.as_bytes();
        if b.len() < DATA_OFFSET {
            return None;
        }
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn dlc(&self) -> Option<u8> {
        self.as_bytes().get(DLC_OFFSET).copied()
    }

    fn data_bytes(&self) -> Option<&[u8]> {
        let b = self.as_bytes();
        let dlc = self.dlc()? as usize;
        let end = DATA_OFFSET + dlc;
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

    fn is_rtr(&self) -> bool {
        self.id_word().map_or(false, |w| w & RTR_FLAG != 0)
    }

    fn is_error_frame(&self) -> bool {
        self.id_word().map_or(false, |w| w & ERR_FLAG != 0)
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

        if b.len() < CLASSIC_CAN_MIN_LEN {
            return Err(CanError::BufferTooShort {
                expected: CLASSIC_CAN_MIN_LEN,
                actual: b.len(),
            });
        }

        let word = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);

        // RTR is illegal when data bytes are present
        if word & RTR_FLAG != 0 && word & EFF_FLAG == 0 {
            return Err(CanError::InvalidFlags);
        }

        let dlc = b[DLC_OFFSET];
        if dlc > CLASSIC_CAN_MAX_DLC {
            return Err(CanError::InvalidDlc(dlc));
        }

        let expected_len = DATA_OFFSET + dlc as usize;
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

    /// Returns the PCI byte offset given the addressing mode.
    fn pci_offset(mode: &AddressMode) -> usize {
        match mode {
            AddressMode::Physical => 0,   // normal addressing
            AddressMode::Functional => 1, // extended/mixed - first byte is address
        }
    }

    /// Returns the data bytes available for ISO-TP PCI + payload
    /// given the addressing mode, or None if the frame is too short.
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

impl<'a> CanFrameExt for CanFrame<'a> {
    fn as_bytes(&self) -> &[u8] {
        ace_proto::common::RawFrame::as_bytes(self)
    }
}

impl<'a> CanFrameExt for CanFrameMut<'a> {
    fn as_bytes(&self) -> &[u8] {
        ace_proto::common::RawFrame::as_bytes(self)
    }
}

// endregion: CanFrameExt

// region: CanFrameMutExt

pub trait CanFrameMutExt: CanFrameExt {
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

    fn set_dlc(&mut self, dlc: u8) -> Result<(), CanError> {
        if dlc > CLASSIC_CAN_MAX_DLC {
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
        if data.len() > CLASSIC_CAN_MAX_DLC as usize {
            return Err(CanError::InvalidDlc(data.len() as u8));
        }
        let b = self.as_bytes_mut();
        let end = DATA_OFFSET + data.len();
        if b.len() < end {
            return Err(CanError::BufferTooShort {
                expected: end,
                actual: b.len(),
            });
        }
        b[DATA_OFFSET..end].copy_from_slice(data);
        b[DLC_OFFSET] = data.len() as u8;
        Ok(())
    }
}

impl<'a> CanFrameMutExt for CanFrameMut<'a> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        ace_proto::common::RawFrameMut::as_bytes_mut(self)
    }
}

// endregion: CanFrameMutExt
