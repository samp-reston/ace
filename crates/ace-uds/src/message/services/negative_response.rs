use crate::{message::ServiceIdentifier, UdsError};
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct NegativeResponse {
    pub request_sid: ServiceIdentifier,
    pub response_code: NegativeResponseCode,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum NegativeResponseCode {
    #[frame(id = 0x00)]
    PositiveResponse,
    #[frame(id = 0x10)]
    GeneralReject,
    #[frame(id = 0x11)]
    ServiceNotSupported,
    #[frame(id = 0x12)]
    SubFunctionNotSupported,
    #[frame(id = 0x13)]
    IncorrectMessageLengthOrInvalidFormat,
    #[frame(id = 0x14)]
    ReponseTooLong,
    #[frame(id = 0x21)]
    BusyRepeatRequest,
    #[frame(id = 0x22)]
    ConditionsNotCorrect,
    #[frame(id = 0x24)]
    RequestSequenceError,
    #[frame(id = 0x25)]
    NoResponseFromSubnetComponent,
    #[frame(id = 0x26)]
    FailurePreventsExecutionOfRequestedAction,
    #[frame(id = 0x31)]
    RequestOutOfRange,
    #[frame(id = 0x33)]
    SecurityAccessDenied,
    #[frame(id = 0x34)]
    AuthenticationRequired,
    #[frame(id = 0x35)]
    InvalidKey,
    #[frame(id = 0x36)]
    ExceedNumberOfAttempts,
    #[frame(id = 0x37)]
    RequiredTimeDelayNotExpired,
    #[frame(id = 0x38)]
    SecureDataTransmissionRequired,
    #[frame(id = 0x39)]
    SecureDataTransmissionNotAllowed,
    #[frame(id = 0x3A)]
    SecureDataVerificationFailed,
    #[frame(id = 0x50)]
    CertificateVerificationFailedInvalidTimePeriod,
    #[frame(id = 0x51)]
    CertificateVerificationFailedInvalidSignature,
    #[frame(id = 0x52)]
    CertificateVerificationFailedInvalidChainOfTrust,
    #[frame(id = 0x53)]
    CertificateVerificationFailedInvalidType,
    #[frame(id = 0x54)]
    CertificateVerificationFailedInvalidFormat,
    #[frame(id = 0x55)]
    CertificateVerificationFailedInvalidContent,
    #[frame(id = 0x56)]
    CertificateVerificationFailedInvalidScope,
    #[frame(id = 0x57)]
    CertificateVerificationFailedInvalidCertificateRevoked,
    #[frame(id = 0x58)]
    OwnershipVerificationFailed,
    #[frame(id = 0x59)]
    ChallengeCalculationFailed,
    #[frame(id = 0x5A)]
    SettingAccessRightsFailed,
    #[frame(id = 0x5B)]
    SessionKeyCreationDerivationFailed,
    #[frame(id = 0x5C)]
    ConfigurationDataUsageFailed,
    #[frame(id = 0x5D)]
    DeAuthenticationFailed,
    #[frame(id = 0x70)]
    UploadDownloadNotAccepted,
    #[frame(id = 0x71)]
    TransferDataSuspended,
    #[frame(id = 0x72)]
    GeneralProgrammingFailure,
    #[frame(id = 0x73)]
    WrongBlockSequenceCounter,
    #[frame(id = 0x78)]
    RequestCorrectlyReceivedResponsePending,
    #[frame(id = 0x7E)]
    SubFunctionNotSupportedInActiveSession,
    #[frame(id = 0x7F)]
    ServiceNotSupportedInActiveSession,
    #[frame(id = 0x81)]
    RPMTooHigh,
    #[frame(id = 0x82)]
    RPMTooLow,
    #[frame(id = 0x83)]
    EngineIsRunning,
    #[frame(id = 0x84)]
    EngineIsNotRunning,
    #[frame(id = 0x85)]
    EngineRunTimeTooLow,
    #[frame(id = 0x86)]
    TemperatureTooHigh,
    #[frame(id = 0x87)]
    TemperatureTooLow,
    #[frame(id = 0x88)]
    VehicleSpeedTooHigh,
    #[frame(id = 0x89)]
    VehicleSpeedTooLow,
    #[frame(id = 0x8A)]
    ThrottlePedalTooHigh,
    #[frame(id = 0x8B)]
    ThrottlePedalTooLow,
    #[frame(id = 0x8C)]
    TransmissionRangeNotInNeutral,
    #[frame(id = 0x8D)]
    TransmissionRangeNotInGear,
    #[frame(id = 0x8F)]
    BrakeSwitchesNotClosed,
    #[frame(id = 0x90)]
    ShifterLeverNotInPark,
    #[frame(id = 0x91)]
    TorqueConverterClutchLocked,
    #[frame(id = 0x92)]
    VoltageTooHigh,
    #[frame(id = 0x93)]
    VoltageTooLow,
    #[frame(id = 0x94)]
    ResourceTemporarilyNotAvailable,
}
