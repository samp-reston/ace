use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = "UdsError")]
#[repr(u8)]
pub enum ServiceIdentifier {
    #[frame(
        id_pat = "0x00 | 0x3F | 0x40 | 0x80..=0x83 | 0x89..=0xB9 | 0xBF..=0xC2 | 0xC9..=0xF9 | 0xFF"
    )]
    NotApplicable(u8),

    #[frame(id_pat = "0x01..=0x0F")]
    EmissionsSpecificServiceRequest(u8),

    #[frame(id_pat = "0x10..=0x3E", decode_inner)]
    UdsServiceRequest(UdsService),

    #[frame(id_pat = "0x41..=0x4F")]
    EmissionsSpecificServicePositiveResponse(u8),

    #[frame(id_pat = "0x50..=0x7E")]
    UdsServicePositiveResponse(u8),

    #[frame(id = "0x7F")]
    NegativeResponse,

    #[frame(id_pat = "0x84..=0x88")]
    ServiceRequests(u8),

    #[frame(id_pat = "0xBA..=0xBE")]
    SystemSupplierServiceRequests(u8),

    #[frame(id_pat = "0xC3..=0xC8")]
    ServiceRequestsPositiveResponse(u8),

    #[frame(id_pat = "0xFA..=0xFE")]
    SystemSupplierServiceRequestsPositiveResponse(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = "UdsError")]
#[repr(u8)]
pub enum UdsService {
    #[frame(id = 0x10)]
    DiagnosticSessionControlRequest = 0x10,
    #[frame(id = 0x50)]
    DiagnosticSessionControlResponse = 0x50,
    #[frame(id = 0x11)]
    ECUResetRequest = 0x11,
    #[frame(id = 0x51)]
    ECUResetResponse = 0x51,
    #[frame(id = 0x27)]
    SecurityAccessRequest = 0x27,
    #[frame(id = 0x67)]
    SecurityAccessResponse = 0x67,
    #[frame(id = 0x28)]
    CommunicationControlRequest = 0x28,
    #[frame(id = 0x68)]
    CommunicationControlResponse = 0x68,
    #[frame(id = 0x29)]
    AuthenticationRequest = 0x29,
    #[frame(id = 0x69)]
    AuthenticationResponse = 0x69,
    #[frame(id = 0x3e)]
    TesterPresentRequest = 0x3e,
    #[frame(id = 0x7e)]
    TesterPresentResponse = 0x7e,
    #[frame(id = 0x85)]
    ControlDTCSettingRequest = 0x85,
    #[frame(id = 0xc5)]
    ControlDTCSettingResponse = 0xc5,
    #[frame(id = 0x86)]
    ResponseOnEventRequest = 0x86,
    #[frame(id = 0xc6)]
    ResponseOnEventResponse = 0xc6,
    #[frame(id = 0x87)]
    LinkControlRequest = 0x87,
    #[frame(id = 0xc7)]
    LinkControlResponse = 0xc7,
    #[frame(id = 0x22)]
    ReadDataByIdentifierRequest = 0x22,
    #[frame(id = 0x62)]
    ReadDataByIdentifierResponse = 0x62,
    #[frame(id = 0x23)]
    ReadMemoryByAddressRequest = 0x23,
    #[frame(id = 0x63)]
    ReadMemoryByAddressResponse = 0x63,
    #[frame(id = 0x24)]
    ReadScalingDataByIdentifierRequest = 0x24,
    #[frame(id = 0x64)]
    ReadScalingDataByIdentifierResponse = 0x64,
    #[frame(id = 0x2a)]
    ReadDataByPeriodicIdentifierRequest = 0x2a,
    #[frame(id = 0x6a)]
    ReadDataByPeriodicIdentifierResponse = 0x6a,
    #[frame(id = 0x2c)]
    DynamicallyDefineDataIdentifierRequest = 0x2c,
    #[frame(id = 0x6c)]
    DynamicallyDefineDataIdentifierResponse = 0x6c,
    #[frame(id = 0x2e)]
    WriteDataByIdentifierRequest = 0x2e,
    #[frame(id = 0x6e)]
    WriteDataByIdentifierResponse = 0x6e,
    #[frame(id = 0x3d)]
    WriteMemoryByAddressRequest = 0x3d,
    #[frame(id = 0x7d)]
    WriteMemoryByAddressResponse = 0x7d,
    #[frame(id = 0x14)]
    ClearDiagnosticInformationRequest = 0x14,
    #[frame(id = 0x54)]
    ClearDiagnosticInformationResponse = 0x54,
    #[frame(id = 0x19)]
    ReadDTCInformationRequest = 0x19,
    #[frame(id = 0x59)]
    ReadDTCInformationResponse = 0x59,
    #[frame(id = 0x2f)]
    InputOutputControlByIdentifierRequest = 0x2f,
    #[frame(id = 0x6f)]
    InputOutputControlByIdentifierResponse = 0x6f,
    #[frame(id = 0x31)]
    RoutineControlRequest = 0x31,
    #[frame(id = 0x71)]
    RoutineControlResponse = 0x71,
    #[frame(id = 0x34)]
    RequestDownloadRequest = 0x34,
    #[frame(id = 0x74)]
    RequestDownloadResponse = 0x74,
    #[frame(id = 0x35)]
    RequestUploadRequest = 0x35,
    #[frame(id = 0x75)]
    RequestUploadResponse = 0x75,
    #[frame(id = 0x36)]
    TransferDataRequest = 0x36,
    #[frame(id = 0x76)]
    TransferDataResponse = 0x76,
    #[frame(id = 0x37)]
    RequestTransferExitRequest = 0x37,
    #[frame(id = 0x77)]
    RequestTransferExitResponse = 0x77,
    #[frame(id = 0x38)]
    RequestFileTransferRequest = 0x38,
    #[frame(id = 0x78)]
    RequestFileTransferResponse = 0x78,
    #[frame(id = 0x84)]
    SecuredDataTransmissionRequest = 0x84,
    #[frame(id = 0xc4)]
    SecuredDataTransmissionResponse = 0xc4,
    #[frame(id = 0x7f)]
    NegativeResponse = 0x7f,
}

impl ServiceIdentifier {
    pub fn discriminant(&self) -> u8 {
        match self {
            ServiceIdentifier::UdsServiceRequest(s) => *s as u8,
            ServiceIdentifier::NegativeResponse => 0x7F,
            ServiceIdentifier::NotApplicable(v)
            | ServiceIdentifier::EmissionsSpecificServiceRequest(v)
            | ServiceIdentifier::EmissionsSpecificServicePositiveResponse(v)
            | ServiceIdentifier::UdsServicePositiveResponse(v)
            | ServiceIdentifier::ServiceRequests(v)
            | ServiceIdentifier::SystemSupplierServiceRequests(v)
            | ServiceIdentifier::ServiceRequestsPositiveResponse(v)
            | ServiceIdentifier::SystemSupplierServiceRequestsPositiveResponse(v) => *v,
        }
    }
    /// Returns `true` if this service defines a sub-function byte at offset 1.
    ///
    /// Per ISO 14229, the following services carry a sub-function byte.
    /// Services excluded are those whose second byte is a data parameter
    /// rather than a sub-function - for example `ReadDataByIdentifier`
    /// uses a 2-byte DID, not a sub-function.
    #[must_use]
    pub fn has_sub_function(&self) -> bool {
        matches!(
            self,
            ServiceIdentifier::UdsServiceRequest(UdsService::DiagnosticSessionControlRequest)
                | ServiceIdentifier::UdsServiceRequest(
                    UdsService::DiagnosticSessionControlResponse
                )
                | ServiceIdentifier::UdsServiceRequest(UdsService::ECUResetRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ECUResetResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::SecurityAccessRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::SecurityAccessResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::CommunicationControlRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::CommunicationControlResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::AuthenticationRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::AuthenticationResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::TesterPresentRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::TesterPresentResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ControlDTCSettingRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ControlDTCSettingResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ResponseOnEventRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ResponseOnEventResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::LinkControlRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::LinkControlResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ReadDTCInformationRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::ReadDTCInformationResponse)
                | ServiceIdentifier::UdsServiceRequest(UdsService::RoutineControlRequest)
                | ServiceIdentifier::UdsServiceRequest(UdsService::RoutineControlResponse)
        )
    }
}
