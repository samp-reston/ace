// region: DoipError

use ace_core::DiagError;
use ace_uds::UdsError;

use crate::header::ProtocolVersion;

#[derive(Debug, PartialEq, Eq)]
pub enum DoipValidationError {
    /// Protocol version byte did not match 0x02 (ISO 13400-2:2019) or 0x01 (ISO 13400-2:2012)
    UnsupportedProtocolVersion(u8),
    /// Inverse protocol version byte did not match the bitwise complement of the version byte
    InvalidInverseProtocolVersion {
        version: ProtocolVersion,
        inverse: u8,
    },
    /// Payload type is not defined in ISO 13400-2
    UnknownPayloadType(u16),
    /// Payload length field does not match the actual number of bytes following the header
    PayloadLengthMismatch { declared: u32, actual: usize },
    /// Frame is shorter than the minimum DoIP header length (8 bytes)
    FrameTooShort { actual: usize },
    /// Source address is not valid in this context (e.g. zero on a response)
    InvalidSourceAddress(u16),
    /// Target address is not valid in this context
    InvalidTargetAddress(u16),
    /// Activation type is not recognised
    UnknownActivationType(u8),
    /// NACK code is not recognised
    UnknownNackCode(u8),
}

#[derive(Debug)]
pub enum DoipError {
    /// Underlying transport or framing error
    Transport(DiagError),
    /// Frame structure violated ISO 13400-2 constraints
    Validation(DoipValidationError),
    /// Frame was structurally valid but could not be parsed
    Parse(heapless::String<64>),
}

impl From<DiagError> for DoipError {
    fn from(e: DiagError) -> Self {
        DoipError::Transport(e)
    }
}

impl From<DoipValidationError> for DoipError {
    fn from(e: DoipValidationError) -> Self {
        DoipError::Validation(e)
    }
}

impl From<UdsError> for DoipError {
    fn from(e: UdsError) -> Self {
        DoipError::Parse(ace_core::diag_err_str(&format!("{:?}", e)))
    }
}

// DoipError does NOT implement Into<DiagError> - the two error domains are
// parallel. DiagError is transport-layer, DoipError is protocol-layer.

// endregion: DoipError
