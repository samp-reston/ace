// region: DoipError

use ace_core::CodecError;
use ace_uds::UdsError;
use heapless::format;

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

    /// Frame is larger than the maximum allowed length
    FrameTooLarge { actual: usize, limit: usize },

    /// Source address is not valid in this context (e.g. zero on a response)
    InvalidSourceAddress(u16),

    /// Target address is not valid in this context
    InvalidTargetAddress(u16),

    /// Activation type is not recognised
    UnknownActivationType(u8),

    /// NACK code is not recognised
    UnknownNackCode(u8),

    /// A wrapper for any casts from u8/16/32/64 -> usize
    ConversionFailure,
}

#[derive(Debug)]
pub enum DoipError {
    /// Frame could not be encoded or decoded
    Codec(CodecError),
    /// Frame structure violated ISO 13400-2 constraints
    Validation(DoipValidationError),
    /// Frame was structurally valid but could not be parsed
    Parse(heapless::String<64>),

    /// The pending-action queue is full, so a required action could not be recorded.
    ///
    /// Distinct from a codec or validation failure: the frame was fine, but the
    /// connection cannot express what must happen next. Dropping the action instead
    /// would desynchronise the caller from the state machine - a lost `Close` leaves
    /// the connection open, a lost `SendDiagnosticNack` leaves the tester waiting.
    ActionQueueFull,
}

impl From<CodecError> for DoipError {
    fn from(e: CodecError) -> Self {
        DoipError::Codec(e)
    }
}

impl From<DoipValidationError> for DoipError {
    fn from(e: DoipValidationError) -> Self {
        DoipError::Validation(e)
    }
}

impl From<UdsError> for DoipError {
    fn from(e: UdsError) -> Self {
        DoipError::Parse(format!("{:?}", e).unwrap_or_default())
    }
}

// DoipError does NOT implement Into<CodecError> - the two error domains are
// parallel. CodecError is transport-layer, DoipError is protocol-layer.

// endregion: DoipError
