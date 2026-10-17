use crate::message::{
    service::{UdsServiceRequest, UdsServiceResponse},
    *,
};

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
    ReadDTCInformationResponse(ReadDtcInformationResponse<'a>),
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
                    UdsServiceRequest::DiagnosticSessionControl,
                ) => Ok(UdsPayload::DiagnosticSessionControlRequest(
                    DiagnosticSessionControlRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::DiagnosticSessionControl,
                ) => Ok(UdsPayload::DiagnosticSessionControlResponse(
                    DiagnosticSessionControlResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::EcuReset) => {
                    Ok(UdsPayload::EcuResetRequest(EcuResetRequest::decode(buf)?))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::EcuReset) => {
                    Ok(UdsPayload::EcuResetResponse(EcuResetResponse::decode(buf)?))
                }
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::SecurityAccess) => Ok(
                    UdsPayload::SecurityAccessRequest(SecurityAccessRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::SecurityAccess) => Ok(
                    UdsPayload::SecurityAccessResponse(SecurityAccessResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::CommunicationControl) => {
                    Ok(UdsPayload::CommunicationControlRequest(
                        CommunicationControlRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::CommunicationControl) => {
                    Ok(UdsPayload::CommunicationControlResponse(
                        CommunicationControlResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::Authentication) => Ok(
                    UdsPayload::AuthenticationRequest(AuthenticationRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::Authentication) => Ok(
                    UdsPayload::AuthenticationResponse(AuthenticationResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::TesterPresent) => Ok(
                    UdsPayload::TesterPresentRequest(TesterPresentRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::TesterPresent) => Ok(
                    UdsPayload::TesterPresentResponse(TesterPresentResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::ControlDtcSetting) => Ok(
                    UdsPayload::ControlDTCSettingRequest(ControlDTCSettingRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::ControlDtcSetting) => Ok(
                    UdsPayload::ControlDTCSettingResponse(ControlDTCSettingResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::ResponseOnEvent) => Ok(
                    UdsPayload::ResponseOnEventRequest(ResponseOnEventRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::ResponseOnEvent) => Ok(
                    UdsPayload::ResponseOnEventResponse(ResponseOnEventResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::LinkControl) => Ok(
                    UdsPayload::LinkControlRequest(LinkControlRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::LinkControl) => Ok(
                    UdsPayload::LinkControlResponse(LinkControlResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(
                    UdsServiceRequest::SecuredDataTransmission,
                ) => Ok(UdsPayload::SecuredDataTransmissionRequest(
                    SecuredDataTransmissionRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::SecuredDataTransmission,
                ) => Ok(UdsPayload::SecuredDataTransmissionResponse(
                    SecuredDataTransmissionResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RoutineControl) => Ok(
                    UdsPayload::RoutineControlRequest(RoutineControlRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::RoutineControl) => Ok(
                    UdsPayload::RoutineControlResponse(RoutineControlResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(
                    UdsServiceRequest::InputOutputControlByIdentifier,
                ) => Ok(UdsPayload::InputOutputControlByIdentifierRequest(
                    InputOutputControlByIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::InputOutputControlByIdentifier,
                ) => Ok(UdsPayload::InputOutputControlByIdentifierResponse(
                    InputOutputControlByIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::ReadDataByIdentifier) => {
                    Ok(UdsPayload::ReadDataByIdentifierRequest(
                        ReadDataByIdentifierRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::ReadDataByIdentifier) => {
                    Ok(UdsPayload::ReadDataByIdentifierResponse(
                        ReadDataByIdentifierResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::ReadMemoryByAddress) => {
                    Ok(UdsPayload::ReadMemoryByAddressRequest(
                        ReadMemoryByAddressRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::ReadMemoryByAddress) => {
                    Ok(UdsPayload::ReadMemoryByAddressResponse(
                        ReadMemoryByAddressResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(
                    UdsServiceRequest::ReadScalingDataByIdentifier,
                ) => Ok(UdsPayload::ReadScalingDataByIdentifierRequest(
                    ReadScalingDataByIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::ReadScalingDataByIdentifier,
                ) => Ok(UdsPayload::ReadScalingDataByIdentifierResponse(
                    ReadScalingDataByIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsServiceRequest::ReadDataByPeriodicIdentifier,
                ) => Ok(UdsPayload::ReadDataByPeriodicIdentifierRequest(
                    ReadDataByPeriodicIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::ReadDataByPeriodicIdentifier,
                ) => Ok(UdsPayload::ReadDataByPeriodicIdentifierResponse(
                    ReadDataByPeriodicIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(
                    UdsServiceRequest::DynamicallyDefineDataIdentifier,
                ) => Ok(UdsPayload::DynamicallyDefineDataIdentifierRequest(
                    DynamicallyDefineDataIdentifierRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::DynamicallyDefineDataIdentifier,
                ) => Ok(UdsPayload::DynamicallyDefineDataIdentifierResponse(
                    DynamicallyDefineDataIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::WriteDataByIdentifier) => {
                    Ok(UdsPayload::WriteDataByIdentifierRequest(
                        WriteDataByIdentifierRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::WriteDataByIdentifier,
                ) => Ok(UdsPayload::WriteDataByIdentifierResponse(
                    WriteDataByIdentifierResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::WriteMemoryByAddress) => {
                    Ok(UdsPayload::WriteMemoryByAddressRequest(
                        WriteMemoryByAddressRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::WriteMemoryByAddress) => {
                    Ok(UdsPayload::WriteMemoryByAddressResponse(
                        WriteMemoryByAddressResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(
                    UdsServiceRequest::ClearDiagnosticInformation,
                ) => Ok(UdsPayload::ClearDiagnosticInformationRequest(
                    ClearDiagnosticInformationRequest::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceResponse(
                    UdsServiceResponse::ClearDiagnosticInformation,
                ) => Ok(UdsPayload::ClearDiagnosticInformationResponse(
                    ClearDiagnosticInformationResponse::decode(buf)?,
                )),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::ReadDTCInformation) => Ok(
                    UdsPayload::ReadDTCInformationRequest(ReadDtcInformationRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::ReadDtcInformation) => {
                    Ok(UdsPayload::ReadDTCInformationResponse(
                        ReadDtcInformationResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestDownload) => Ok(
                    UdsPayload::RequestDownloadRequest(RequestDownloadRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::RequestDownload) => Ok(
                    UdsPayload::RequestDownloadResponse(RequestDownloadResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestUpload) => Ok(
                    UdsPayload::RequestUploadRequest(RequestUploadRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::RequestUpload) => Ok(
                    UdsPayload::RequestUploadResponse(RequestUploadResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::TransferData) => Ok(
                    UdsPayload::TransferDataRequest(TransferDataRequest::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::TransferData) => Ok(
                    UdsPayload::TransferDataResponse(TransferDataResponse::decode(buf)?),
                ),
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestTransferExit) => {
                    Ok(UdsPayload::RequestTransferExitRequest(
                        RequestTransferExitRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::RequestTransferExit) => {
                    Ok(UdsPayload::RequestTransferExitResponse(
                        RequestTransferExitResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceRequest(UdsServiceRequest::RequestFileTransfer) => {
                    Ok(UdsPayload::RequestFileTransferRequest(
                        RequestFileTransferRequest::decode(buf)?,
                    ))
                }
                ServiceIdentifier::UdsServiceResponse(UdsServiceResponse::RequestFileTransfer) => {
                    Ok(UdsPayload::RequestFileTransferResponse(
                        RequestFileTransferResponse::decode(buf)?,
                    ))
                }
                ServiceIdentifier::NegativeResponse => {
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
