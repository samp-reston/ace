// region: NRC Error

/// A trait for application error types that can be converted into UDS Negative Response Codes
/// (NRCs).
///
/// The server state machine uses this to construct NRC responses without knowing the application's
/// error type internals. The application defines the mapping via `Into<u8>`.
pub trait NrcError: Into<u8> + core::fmt::Debug {
    // region: Mandatory constructors - one per NRC the server may emit

    /// 0x11 - serviceNotSupported
    fn service_not_supported() -> Self;
    /// 0x12 - subFunctionNotSupported
    fn sub_function_not_supported() -> Self;
    /// 0x13 - incorrectMessageLengthOrInvalidFormat
    fn incorrect_message_length_or_invalid_format() -> Self;
    /// 0x14 - responseTooLong
    ///
    /// Emitted when a handler returns more data than the response frame can carry.
    /// Without this the server has no conformant way to report the condition, and
    /// its only alternatives are a truncated positive response or silence.
    fn response_too_long() -> Self;
    /// 0x22 - conditionsNotCorrect
    fn conditions_not_correct() -> Self;
    /// 0x24 - requestSequenceError
    fn request_sequence_error() -> Self;
    /// 0x31 - requestOutOfRange
    fn request_out_of_range() -> Self;
    /// 0x33 - securityAccessDenied
    fn security_access_denied() -> Self;
    /// 0x35 - invalidKey
    fn invalid_key() -> Self;
    /// 0x36 - exceededNumberOfAttempts
    fn exceeded_number_of_attempts() -> Self;
    /// 0x37 - requiredTimeDelayNotExpired
    fn required_time_delay_not_expired() -> Self;
    /// 0x70 - uploadDownloadNotAccepted
    fn upload_download_not_accepted() -> Self;
    /// 0x71 - transferDataSuspended
    fn transfer_data_suspended() -> Self;
    /// 0x72 - generalProgrammingFailure
    fn general_programming_failure() -> Self;
    /// 0x73 - wrongBlockSequenceCounter
    fn wrong_block_sequence_counter() -> Self;
    /// 0x78 - requestCorrectlyReceivedResponsePending
    fn response_pending() -> Self;

    /// 0x21 - busyRepeatRequest.
    ///
    /// The server cannot take this request now and the client should retry it. Needed
    /// because UDS allows one request at a time on a physical connection: a second request
    /// arriving while a response is still pending has to be refused, and refusing it with
    /// anything else would tell the client something untrue about the request itself.
    fn busy_repeat_request() -> Self;
    /// 0x7E - subFunctionNotSupportedInActiveSession
    fn sub_function_not_supported_in_active_session() -> Self;
    /// 0x7F - serviceNotSupportedInActiveSession
    fn service_not_supported_in_active_session() -> Self;

    // endregion: Mandatory constructors
}

// endregion: NRC Error

// region: Builtin NRC

/// A built-in NRC type covering all codes the server may emit.
///
/// Applications that do not need a custom error type may use this directly as `type Error =
/// BuiltinNrc` in the `ServerHandler` impl.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum BuiltinNrc {
    ServiceNotSupported = 0x11,
    SubFunctionNotSupported = 0x12,
    IncorrectMessageLengthOrInvalidFormat = 0x13,
    ResponseTooLong = 0x14,
    ConditionsNotCorrect = 0x22,
    RequestSequenceError = 0x24,
    RequestOutOfRange = 0x31,
    SecurityAccessDenied = 0x33,
    InvalidKey = 0x35,
    ExceededNumberOfAttempts = 0x36,
    RequiredTimeDelayNotExpired = 0x37,
    UploadDownloadNotAccepted = 0x70,
    TransferDataSuspended = 0x71,
    GeneralProgrammingFailure = 0x72,
    WrongBlockSequenceCounter = 0x73,
    ResponsePending = 0x78,
    BusyRepeatRequest = 0x21,
    SubFunctionNotSupportedInActiveSession = 0x7E,
    ServiceNotSupportedInActiveSession = 0x7F,
}

impl From<BuiltinNrc> for u8 {
    fn from(n: BuiltinNrc) -> u8 {
        n as u8
    }
}

impl NrcError for BuiltinNrc {
    fn service_not_supported() -> Self {
        Self::ServiceNotSupported
    }
    fn sub_function_not_supported() -> Self {
        Self::SubFunctionNotSupported
    }
    fn incorrect_message_length_or_invalid_format() -> Self {
        Self::IncorrectMessageLengthOrInvalidFormat
    }
    fn response_too_long() -> Self {
        Self::ResponseTooLong
    }
    fn conditions_not_correct() -> Self {
        Self::ConditionsNotCorrect
    }
    fn request_sequence_error() -> Self {
        Self::RequestSequenceError
    }
    fn request_out_of_range() -> Self {
        Self::RequestOutOfRange
    }
    fn security_access_denied() -> Self {
        Self::SecurityAccessDenied
    }
    fn invalid_key() -> Self {
        Self::InvalidKey
    }
    fn exceeded_number_of_attempts() -> Self {
        Self::ExceededNumberOfAttempts
    }
    fn required_time_delay_not_expired() -> Self {
        Self::RequiredTimeDelayNotExpired
    }
    fn upload_download_not_accepted() -> Self {
        Self::UploadDownloadNotAccepted
    }
    fn transfer_data_suspended() -> Self {
        Self::TransferDataSuspended
    }
    fn general_programming_failure() -> Self {
        Self::GeneralProgrammingFailure
    }
    fn wrong_block_sequence_counter() -> Self {
        Self::WrongBlockSequenceCounter
    }
    fn response_pending() -> Self {
        Self::ResponsePending
    }

    fn busy_repeat_request() -> Self {
        Self::BusyRepeatRequest
    }
    fn sub_function_not_supported_in_active_session() -> Self {
        Self::SubFunctionNotSupportedInActiveSession
    }
    fn service_not_supported_in_active_session() -> Self {
        Self::ServiceNotSupportedInActiveSession
    }
}

// endregion: Builtin NRC
