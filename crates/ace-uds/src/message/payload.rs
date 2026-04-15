use crate::message::{service::UdsService, *};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UdsPayload<'a> {
    DiagnosticSessionControlRequest(DiagnosticSessionControlRequest),
    DiagnosticSessionControlResponse(DiagnosticSessionControlResponse),
    RequestTransferExitRequest(RequestTransferExitRequest<'a>),
    RequestTransferExitResponse(RequestTransferExitResponse<'a>),
    TransferDataRequest(TransferDataRequest<'a>),
    TransferDataResponse(TransferDataResponse<'a>),
    EcuResetRequest(EcuResetRequest),
    EcuResetResponse(EcuResetResponse),
    RequestFileTransferRequest(RequestFileTransferRequest<'a>),
    RequestFileTransferResponse(RequestFileTransferResponse<'a>),
    SecurityAccessRequest(SecurityAccessRequest<'a>),
    SecurityAccessResponse(SecurityAccessResponse<'a>),
    CommunicationControlRequest(CommunicationControlRequest),
    CommunicationControlResponse(CommunicationControlResponse),
    AuthenticationRequest(AuthenticationRequest<'a>),
    AuthenticationResponse(AuthenticationResponse<'a>),
    TesterPresentRequest(TesterPresentRequest),
    TesterPresentResponse(TesterPresentResponse),
    ControlDTCSettingRequest(ControlDTCSettingRequest<'a>),
    ControlDTCSettingResponse(ControlDTCSettingResponse),
    ResponseOnEventRequest(ResponseOnEventRequest<'a>),
    ResponseOnEventResponse(ResponseOnEventResponse<'a>),
    LinkControlRequest(LinkControlRequest),
    LinkControlResponse(LinkControlResponse),
    SecuredDataTransmissionRequest(SecuredDataTransmissionRequest<'a>),
    SecuredDataTransmissionResponse(SecuredDataTransmissionResponse<'a>),
    RoutineControlRequest(RoutineControlRequest<'a>),
    RoutineControlResponse(RoutineControlResponse<'a>),
    InputOutputControlByIdentifierRequest(InputOutputControlByIdentifierRequest<'a>),
    InputOutputControlByIdentifierResponse(InputOutputControlByIdentifierResponse<'a>),
    ReadDataByIdentifierRequest(ReadDataByIdentifierRequest<'a>),
    ReadDataByIdentifierResponse(ReadDataByIdentifierResponse<'a>),
    ReadMemoryByAddressRequest(ReadMemoryByAddressRequest<'a>),
    ReadMemoryByAddressResponse(ReadMemoryByAddressResponse<'a>),
    ReadScalingDataByIdentifierRequest(ReadScalingDataByIdentifierRequest<'a>),
    ReadScalingDataByIdentifierResponse(ReadScalingDataByIdentifierResponse<'a>),
    ReadDataByPeriodicIdentifierRequest(ReadDataByPeriodicIdentifierRequest<'a>),
    ReadDataByPeriodicIdentifierResponse(ReadDataByPeriodicIdentifierResponse),
    ReadDataByPeriodicIdentifierResponseData(ReadDataByPeriodicIdentifierResponseData<'a>),
    DynamicallyDefineDataIdentifierRequest(DynamicallyDefineDataIdentifierRequest<'a>),
    DynamicallyDefineDataIdentifierResponse(DynamicallyDefineDataIdentifierResponse),
    WriteDataByIdentifierRequest(WriteDataByIdentifierRequest<'a>),
    WriteDataByIdentifierResponse(WriteDataByIdentifierResponse),
    WriteMemoryByAddressRequest(WriteMemoryByAddressRequest<'a>),
    WriteMemoryByAddressResponse(WriteMemoryByAddressResponse<'a>),
    ReadDTCInformationRequest(ReadDtcInformationRequest),
    ReadDTCInformationResponse(ReadDTCInformationResponse<'a>),
    RequestDownloadRequest(RequestDownloadRequest<'a>),
    RequestDownloadResponse(RequestDownloadResponse<'a>),
    RequestUploadRequest(RequestUploadRequest<'a>),
    RequestUploadResponse(RequestUploadResponse<'a>),
    NegativeResponse(NegativeResponse),
    ClearDiagnosticInformationRequest(ClearDiagnosticInformationRequest),
    ClearDiagnosticInformationResponse(ClearDiagnosticInformationResponse),
}

// region: Payload codec

impl<'a> UdsPayload<'a> {
    pub fn decode(sid: Option<ServiceIdentifier>, buf: &mut &'a [u8]) -> Result<Self, UdsError> {
        match sid {
            None => Ok(UdsPayload::ReadDataByPeriodicIdentifierResponseData(
                ReadDataByPeriodicIdentifierResponseData::decode(buf)?,
            )),

            Some(sid) => match sid {
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::DiagnosticSessionControlRequest,
                ) => Ok(UdsPayload::DiagnosticSessionControlRequest(
                    DiagnosticSessionControlRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::DiagnosticSessionControlResponse,
                ) => Ok(UdsPayload::DiagnosticSessionControlResponse(
                    DiagnosticSessionControlResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsService::ECUResetRequest) => {
                    Ok(UdsPayload::EcuResetRequest(EcuResetRequest::decode(buf)?))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::ECUResetResponse) => {
                    Ok(UdsPayload::EcuResetResponse(EcuResetResponse::decode(buf)?))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::SecurityAccessRequest) => Ok(
                    UdsPayload::SecurityAccessRequest(SecurityAccessRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::SecurityAccessResponse) => Ok(
                    UdsPayload::SecurityAccessResponse(SecurityAccessResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::CommunicationControlRequest) => {
                    Ok(UdsPayload::CommunicationControlRequest(
                        CommunicationControlRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::CommunicationControlResponse) => {
                    Ok(UdsPayload::CommunicationControlResponse(
                        CommunicationControlResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::AuthenticationRequest) => Ok(
                    UdsPayload::AuthenticationRequest(AuthenticationRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::AuthenticationResponse) => Ok(
                    UdsPayload::AuthenticationResponse(AuthenticationResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::TesterPresentRequest) => Ok(
                    UdsPayload::TesterPresentRequest(TesterPresentRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::TesterPresentResponse) => Ok(
                    UdsPayload::TesterPresentResponse(TesterPresentResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::ControlDTCSettingRequest) => Ok(
                    UdsPayload::ControlDTCSettingRequest(ControlDTCSettingRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::ControlDTCSettingResponse) => Ok(
                    UdsPayload::ControlDTCSettingResponse(ControlDTCSettingResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::ResponseOnEventRequest) => Ok(
                    UdsPayload::ResponseOnEventRequest(ResponseOnEventRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::ResponseOnEventResponse) => Ok(
                    UdsPayload::ResponseOnEventResponse(ResponseOnEventResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::LinkControlRequest) => Ok(
                    UdsPayload::LinkControlRequest(LinkControlRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::LinkControlResponse) => Ok(
                    UdsPayload::LinkControlResponse(LinkControlResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::SecuredDataTransmissionRequest,
                ) => Ok(UdsPayload::SecuredDataTransmissionRequest(
                    SecuredDataTransmissionRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::SecuredDataTransmissionResponse,
                ) => Ok(UdsPayload::SecuredDataTransmissionResponse(
                    SecuredDataTransmissionResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsService::RoutineControlRequest) => Ok(
                    UdsPayload::RoutineControlRequest(RoutineControlRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::RoutineControlResponse) => Ok(
                    UdsPayload::RoutineControlResponse(RoutineControlResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::InputOutputControlByIdentifierRequest,
                ) => Ok(UdsPayload::InputOutputControlByIdentifierRequest(
                    InputOutputControlByIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::InputOutputControlByIdentifierResponse,
                ) => Ok(UdsPayload::InputOutputControlByIdentifierResponse(
                    InputOutputControlByIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsService::ReadDataByIdentifierRequest) => {
                    Ok(UdsPayload::ReadDataByIdentifierRequest(
                        ReadDataByIdentifierRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::ReadDataByIdentifierResponse) => {
                    Ok(UdsPayload::ReadDataByIdentifierResponse(
                        ReadDataByIdentifierResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::ReadMemoryByAddressRequest) => {
                    Ok(UdsPayload::ReadMemoryByAddressRequest(
                        ReadMemoryByAddressRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::ReadMemoryByAddressResponse) => {
                    Ok(UdsPayload::ReadMemoryByAddressResponse(
                        ReadMemoryByAddressResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::ReadScalingDataByIdentifierRequest,
                ) => Ok(UdsPayload::ReadScalingDataByIdentifierRequest(
                    ReadScalingDataByIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::ReadScalingDataByIdentifierResponse,
                ) => Ok(UdsPayload::ReadScalingDataByIdentifierResponse(
                    ReadScalingDataByIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::ReadDataByPeriodicIdentifierRequest,
                ) => Ok(UdsPayload::ReadDataByPeriodicIdentifierRequest(
                    ReadDataByPeriodicIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::ReadDataByPeriodicIdentifierResponse,
                ) => Ok(UdsPayload::ReadDataByPeriodicIdentifierResponse(
                    ReadDataByPeriodicIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::DynamicallyDefineDataIdentifierRequest,
                ) => Ok(UdsPayload::DynamicallyDefineDataIdentifierRequest(
                    DynamicallyDefineDataIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::DynamicallyDefineDataIdentifierResponse,
                ) => Ok(UdsPayload::DynamicallyDefineDataIdentifierResponse(
                    DynamicallyDefineDataIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsService::WriteDataByIdentifierRequest) => {
                    Ok(UdsPayload::WriteDataByIdentifierRequest(
                        WriteDataByIdentifierRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::WriteDataByIdentifierResponse) => {
                    Ok(UdsPayload::WriteDataByIdentifierResponse(
                        WriteDataByIdentifierResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::WriteMemoryByAddressRequest) => {
                    Ok(UdsPayload::WriteMemoryByAddressRequest(
                        WriteMemoryByAddressRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::WriteMemoryByAddressResponse) => {
                    Ok(UdsPayload::WriteMemoryByAddressResponse(
                        WriteMemoryByAddressResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::ClearDiagnosticInformationRequest,
                ) => Ok(UdsPayload::ClearDiagnosticInformationRequest(
                    ClearDiagnosticInformationRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsService::ClearDiagnosticInformationResponse,
                ) => Ok(UdsPayload::ClearDiagnosticInformationResponse(
                    ClearDiagnosticInformationResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsService::ReadDTCInformationRequest) => Ok(
                    UdsPayload::ReadDTCInformationRequest(ReadDtcInformationRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::ReadDTCInformationResponse) => {
                    Ok(UdsPayload::ReadDTCInformationResponse(
                        ReadDTCInformationResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestDownloadRequest) => Ok(
                    UdsPayload::RequestDownloadRequest(RequestDownloadRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestDownloadResponse) => Ok(
                    UdsPayload::RequestDownloadResponse(RequestDownloadResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestUploadRequest) => Ok(
                    UdsPayload::RequestUploadRequest(RequestUploadRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestUploadResponse) => Ok(
                    UdsPayload::RequestUploadResponse(RequestUploadResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::TransferDataRequest) => Ok(
                    UdsPayload::TransferDataRequest(TransferDataRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::TransferDataResponse) => Ok(
                    UdsPayload::TransferDataResponse(TransferDataResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestTransferExitRequest) => {
                    Ok(UdsPayload::RequestTransferExitRequest(
                        RequestTransferExitRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestTransferExitResponse) => {
                    Ok(UdsPayload::RequestTransferExitResponse(
                        RequestTransferExitResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestFileTransferRequest) => {
                    Ok(UdsPayload::RequestFileTransferRequest(
                        RequestFileTransferRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::RequestFileTransferResponse) => {
                    Ok(UdsPayload::RequestFileTransferResponse(
                        RequestFileTransferResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsService::NegativeResponse) => {
                    Ok(UdsPayload::NegativeResponse(NegativeResponse::decode(buf)?))
                }
                _ => Err(UdsError::Validation(ValidationError::UnsupportedService(
                    sid.discriminant(),
                ))),
            },
        }
    }
}

impl FrameWrite for UdsPayload<'_> {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        match self {
            UdsPayload::DiagnosticSessionControlRequest(inner) => inner.encode(buf),
            UdsPayload::DiagnosticSessionControlResponse(inner) => inner.encode(buf),
            UdsPayload::EcuResetRequest(inner) => inner.encode(buf),
            UdsPayload::EcuResetResponse(inner) => inner.encode(buf),
            UdsPayload::SecurityAccessRequest(inner) => inner.encode(buf),
            UdsPayload::SecurityAccessResponse(inner) => inner.encode(buf),
            UdsPayload::CommunicationControlRequest(inner) => inner.encode(buf),
            UdsPayload::CommunicationControlResponse(inner) => inner.encode(buf),
            UdsPayload::AuthenticationRequest(inner) => inner.encode(buf),
            UdsPayload::AuthenticationResponse(inner) => inner.encode(buf),
            UdsPayload::TesterPresentRequest(inner) => inner.encode(buf),
            UdsPayload::TesterPresentResponse(inner) => inner.encode(buf),
            UdsPayload::ControlDTCSettingRequest(inner) => inner.encode(buf),
            UdsPayload::ControlDTCSettingResponse(inner) => inner.encode(buf),
            UdsPayload::ResponseOnEventRequest(inner) => inner.encode(buf),
            UdsPayload::ResponseOnEventResponse(inner) => inner.encode(buf),
            UdsPayload::LinkControlRequest(inner) => inner.encode(buf),
            UdsPayload::LinkControlResponse(inner) => inner.encode(buf),
            UdsPayload::SecuredDataTransmissionRequest(inner) => inner.encode(buf),
            UdsPayload::SecuredDataTransmissionResponse(inner) => inner.encode(buf),
            UdsPayload::RoutineControlRequest(inner) => inner.encode(buf),
            UdsPayload::RoutineControlResponse(inner) => inner.encode(buf),
            UdsPayload::InputOutputControlByIdentifierRequest(inner) => inner.encode(buf),
            UdsPayload::InputOutputControlByIdentifierResponse(inner) => inner.encode(buf),
            UdsPayload::ReadDataByIdentifierRequest(inner) => inner.encode(buf),
            UdsPayload::ReadDataByIdentifierResponse(inner) => inner.encode(buf),
            UdsPayload::ReadMemoryByAddressRequest(inner) => inner.encode(buf),
            UdsPayload::ReadMemoryByAddressResponse(inner) => inner.encode(buf),
            UdsPayload::ReadScalingDataByIdentifierRequest(inner) => inner.encode(buf),
            UdsPayload::ReadScalingDataByIdentifierResponse(inner) => inner.encode(buf),
            UdsPayload::ReadDataByPeriodicIdentifierRequest(inner) => inner.encode(buf),
            UdsPayload::ReadDataByPeriodicIdentifierResponse(inner) => inner.encode(buf),
            UdsPayload::ReadDataByPeriodicIdentifierResponseData(inner) => inner.encode(buf),
            UdsPayload::DynamicallyDefineDataIdentifierRequest(inner) => inner.encode(buf),
            UdsPayload::DynamicallyDefineDataIdentifierResponse(inner) => inner.encode(buf),
            UdsPayload::WriteDataByIdentifierRequest(inner) => inner.encode(buf),
            UdsPayload::WriteDataByIdentifierResponse(inner) => inner.encode(buf),
            UdsPayload::WriteMemoryByAddressRequest(inner) => inner.encode(buf),
            UdsPayload::WriteMemoryByAddressResponse(inner) => inner.encode(buf),
            UdsPayload::ClearDiagnosticInformationRequest(inner) => inner.encode(buf),
            UdsPayload::ClearDiagnosticInformationResponse(inner) => inner.encode(buf),
            UdsPayload::ReadDTCInformationRequest(inner) => inner.encode(buf),
            UdsPayload::ReadDTCInformationResponse(inner) => inner.encode(buf),
            UdsPayload::RequestDownloadRequest(inner) => inner.encode(buf),
            UdsPayload::RequestDownloadResponse(inner) => inner.encode(buf),
            UdsPayload::RequestUploadRequest(inner) => inner.encode(buf),
            UdsPayload::RequestUploadResponse(inner) => inner.encode(buf),
            UdsPayload::TransferDataRequest(inner) => inner.encode(buf),
            UdsPayload::TransferDataResponse(inner) => inner.encode(buf),
            UdsPayload::RequestTransferExitRequest(inner) => inner.encode(buf),
            UdsPayload::RequestTransferExitResponse(inner) => inner.encode(buf),
            UdsPayload::RequestFileTransferRequest(inner) => inner.encode(buf),
            UdsPayload::RequestFileTransferResponse(inner) => inner.encode(buf),
            UdsPayload::NegativeResponse(inner) => inner.encode(buf),
        }
    }
}

// endregion: Payload codec
