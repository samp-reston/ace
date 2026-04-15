use crate::{
    error::DoipValidationError,
    header::{PayloadType, ProtocolVersion},
};
use ace_proto::{
    common::{RawFrame, RawFrameMut},
    doip::constants::{
        DOIP_HEADER_LEN, DOIP_INV_VERSION_OFFSET, DOIP_LENGTH_OFFSET, DOIP_TYPE_OFFSET,
        DOIP_VERSION_OFFSET,
    },
    DoipFrame, DoipFrameMut,
};

// region: DoipFrameExt

pub trait DoipFrameExt {
    fn as_bytes(&self) -> &[u8];

    // region: Raw header field accessors

    fn protocol_version(&self) -> Option<u8> {
        self.as_bytes().get(DOIP_VERSION_OFFSET).copied()
    }

    fn inverse_protocol_version(&self) -> Option<u8> {
        self.as_bytes().get(DOIP_INV_VERSION_OFFSET).copied()
    }

    fn payload_type_raw(&self) -> Option<u16> {
        let b = self.as_bytes();
        if b.len() < DOIP_TYPE_OFFSET {
            return None;
        }
        Some(u16::from_be_bytes([
            b[DOIP_TYPE_OFFSET],
            b[DOIP_TYPE_OFFSET + 1],
        ]))
    }

    fn payload_length_declared(&self) -> Option<u32> {
        let b = self.as_bytes();
        if b.len() < DOIP_HEADER_LEN {
            return None;
        }
        Some(u32::from_be_bytes([
            b[DOIP_LENGTH_OFFSET],
            b[DOIP_LENGTH_OFFSET + 1],
            b[DOIP_LENGTH_OFFSET + 2],
            b[DOIP_LENGTH_OFFSET + 3],
        ]))
    }

    fn payload_bytes(&self) -> Option<&[u8]> {
        let b = self.as_bytes();
        if b.len() < DOIP_HEADER_LEN {
            return None;
        }
        Some(&b[DOIP_HEADER_LEN..])
    }

    // endregion: Raw header field accessors

    // region: Validation

    /// Checks all ISO 13400-2 header invariants and returns the first violation
    /// found, or Ok(()) if the header is well-formed.
    fn validate_header(&self) -> Result<(), DoipValidationError> {
        let b = self.as_bytes();

        // Must have at least 8 bytes to contain a complete DoIP header
        if b.len() < DOIP_HEADER_LEN {
            return Err(DoipValidationError::FrameTooShort { actual: b.len() });
        }

        // Protocol version must be a known ISO 13400-2 value
        let version = ProtocolVersion::try_from(b[DOIP_VERSION_OFFSET])
            .map_err(|_| DoipValidationError::UnsupportedProtocolVersion(b[DOIP_VERSION_OFFSET]))?;

        // Inverse protocol version must be the bitwise complement of the version byte
        let inverse = b[DOIP_INV_VERSION_OFFSET];
        if inverse != !(version as u8) {
            return Err(DoipValidationError::InvalidInverseProtocolVersion { version, inverse });
        }

        // Payload type must be a known ISO 13400-2 value
        let payload_type_raw = u16::from_be_bytes([b[DOIP_TYPE_OFFSET], b[DOIP_TYPE_OFFSET + 1]]);
        PayloadType::try_from(payload_type_raw)
            .map_err(|_| DoipValidationError::UnknownPayloadType(payload_type_raw))?;

        // Declared payload length must match actual bytes following the header
        let declared = u32::from_be_bytes([
            b[DOIP_LENGTH_OFFSET],
            b[DOIP_LENGTH_OFFSET + 1],
            b[DOIP_LENGTH_OFFSET + 2],
            b[DOIP_LENGTH_OFFSET + 3],
        ]) as usize;
        let actual = b.len() - DOIP_HEADER_LEN;

        if declared != actual {
            return Err(DoipValidationError::PayloadLengthMismatch {
                declared: declared as u32,
                actual,
            });
        }

        Ok(())
    }

    /// Returns true if the header passes all ISO 13400-2 invariants.
    fn is_valid(&self) -> bool {
        self.validate_header().is_ok()
    }

    // endregion: Validation

    // region: Typed payload type

    fn payload_type(&self) -> Option<Result<PayloadType, DoipValidationError>> {
        self.payload_type_raw().map(|raw| {
            PayloadType::try_from(raw).map_err(|_| DoipValidationError::UnknownPayloadType(raw))
        })
    }
    // endregion: Typed payload type
}

// endregion: DoipFrameExt

impl<'a> DoipFrameExt for DoipFrame<'a> {
    fn as_bytes(&self) -> &[u8] {
        RawFrame::as_bytes(self)
    }
}

impl<'a> DoipFrameExt for DoipFrameMut<'a> {
    fn as_bytes(&self) -> &[u8] {
        RawFrame::as_bytes(self)
    }
}

impl<'a> DoipFrameMutExt for DoipFrameMut<'a> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        RawFrameMut::as_bytes_mut(self)
    }
}

// region: DoipFrameMutExt

pub trait DoipFrameMutExt: DoipFrameExt {
    fn as_bytes_mut(&mut self) -> &mut [u8];

    fn set_protocol_version(&mut self, version: ProtocolVersion) {
        let v = version as u8;

        if let Some(b) = self.as_bytes_mut().get_mut(DOIP_VERSION_OFFSET) {
            *b = v;
        }

        if let Some(b) = self.as_bytes_mut().get_mut(DOIP_INV_VERSION_OFFSET) {
            *b = !v;
        }
    }

    fn set_payload_type(&mut self, payload_type: PayloadType) {
        let raw = payload_type as u16;
        let bytes = raw.to_be_bytes();

        if let Some(b) = self.as_bytes_mut().get_mut(DOIP_TYPE_OFFSET) {
            *b = bytes[0];
        }

        if let Some(b) = self.as_bytes_mut().get_mut(DOIP_TYPE_OFFSET + 1) {
            *b = bytes[1];
        }
    }

    fn set_payload_length(&mut self, length: u32) {
        let bytes = length.to_be_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if let Some(b) = self.as_bytes_mut().get_mut(DOIP_LENGTH_OFFSET + i) {
                *b = byte;
            }
        }
    }
}

// endregion: DoipFrameMutExt
