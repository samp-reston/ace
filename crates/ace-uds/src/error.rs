// region: UdsError

#[derive(Debug)]
pub enum UdsError {
    Transport(ace_core::DiagError),
    NegativeResponse(u8),
    Parse(heapless::String<64>),
    ResponsePending,
    Validation(ValidationError),
}

// endregion: UdsError

// region: UdsError Display

impl core::fmt::Display for UdsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UdsError::Transport(e) => write!(f, "transport error: {:?}", e),
            UdsError::NegativeResponse(nrc) => {
                write!(f, "negative response: 0x{:02X}", nrc)
            }
            UdsError::Parse(msg) => write!(f, "parse error: {}", msg),
            UdsError::ResponsePending => write!(f, "response pending (NRC 0x78)"),
            UdsError::Validation(e) => write!(f, "validation error: {}", e),
        }
    }
}

// endregion: UdsError Display

// region: UdsError From Impls

impl From<ace_core::DiagError> for UdsError {
    fn from(e: ace_core::DiagError) -> Self {
        UdsError::Transport(e)
    }
}

impl From<ValidationError> for UdsError {
    fn from(e: ValidationError) -> Self {
        UdsError::Validation(e)
    }
}

impl From<UdsError> for ace_core::DiagError {
    fn from(e: UdsError) -> Self {
        match e {
            UdsError::Transport(diag) => diag,
            UdsError::Parse(msg) => ace_core::DiagError::InvalidFrame(msg),
            UdsError::NegativeResponse(nrc) => ace_core::DiagError::InvalidFrame(
                heapless::String::try_from(format!("negative response: {nrc}").as_str())
                    .unwrap_or_default(),
            ),
            UdsError::ResponsePending => ace_core::DiagError::InvalidFrame(
                heapless::String::try_from("response pending").unwrap_or_default(),
            ),
            UdsError::Validation(e) => ace_core::DiagError::InvalidFrame(
                heapless::String::try_from(format!("validation error: {e}").as_str())
                    .unwrap_or_default(),
            ),
        }
    }
}

// endregion: UdsError From Impls

// region: ValidationError

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidEventType(u8),
    UnsupportedService(u8),
    InvalidSubFunction(u8),
    InvalidLength { expected: usize, actual: usize },
    InvalidSessionType(u8),
    InvalidAddressAndLengthFormat(u8),
    InvalidDataIdentifier(u16),
    InvalidSecurityAccessType(u8),
    InvalidCommunicationTypeValue(u8),
    InvalidCommunicationReserved(u8),
    InvalidSubnet(u8),
    ServiceSpecific(heapless::String<64>),
    InvalidDtcGroup(u32),
}

// endregion: ValidationError

// region: ValidationError Display

impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValidationError::UnsupportedService(sid) => {
                write!(f, "invalid service identifier: {:?}", sid)
            }
            ValidationError::InvalidSubFunction(sf) => {
                write!(f, "invalid sub-function: 0x{:02X}", sf)
            }
            ValidationError::InvalidLength { expected, actual } => {
                write!(f, "invalid length: expected {}, got {}", expected, actual)
            }
            ValidationError::InvalidSessionType(st) => {
                write!(f, "invalid session type: 0x{:02X}", st)
            }
            ValidationError::InvalidAddressAndLengthFormat(byte) => {
                write!(
                    f,
                    "invalid addressAndLengthFormatIdentifier: 0x{:02X}",
                    byte
                )
            }
            ValidationError::InvalidDataIdentifier(did) => {
                write!(f, "invalid data identifier: 0x{:04X}", did)
            }
            ValidationError::ServiceSpecific(msg) => {
                write!(f, "service validation error: {}", msg)
            }
            ValidationError::InvalidSecurityAccessType(byte) => {
                write!(f, "invalid securityAccessType: 0x{:02X}", byte)
            }
            ValidationError::InvalidCommunicationReserved(byte) => {
                write!(f, "invalid communicationReserved: 0x{:02X}", byte)
            }
            ValidationError::InvalidSubnet(byte) => {
                write!(f, "invalid subnet: 0x{:02X}", byte)
            }
            ValidationError::InvalidCommunicationTypeValue(byte) => {
                write!(f, "invalid communicationTypeValue: 0x{:02X}", byte)
            }
            ValidationError::InvalidEventType(byte) => {
                write!(f, "invalid eventType: 0x{:02X}", byte)
            }
            ValidationError::InvalidDtcGroup(byte) => {
                write!(f, "invalid dtcGroup: 0x{:02X}", byte)
            }
        }
    }
}

// endregion: ValidationError Display
