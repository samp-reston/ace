#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BufferTooShort { expected: usize, actual: usize },
    InvalidProtocolVersion,
    PayloadLengthMismatch { expected: usize, actual: usize },
    InvalidPayloadType,
    InvalidPayloadLength,
}

impl From<Error> for ace_core::DiagError {
    fn from(e: Error) -> Self {
        match e {
            Error::BufferTooShort { expected, actual } => {
                ace_core::DiagError::LengthMismatch { expected, actual }
            }
            Error::PayloadLengthMismatch { expected, actual } => {
                ace_core::DiagError::LengthMismatch { expected, actual }
            }
            _ => ace_core::DiagError::InvalidFrame(ace_core::diag_err_str("proto parse error")),
        }
    }
}
