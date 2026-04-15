// crates/ace-uds/tests/roundtrip.rs
//
// Round-trip property tests: for any byte sequence that decodes successfully,
// encode → decode must produce the original value.
//
// Tests are organised into two layers:
//   1. Inner structs - each concrete request/response type tested directly.
//   2. Entry points - UdsMessage and Payload tested via the top-level decoder.
//
// Proptest strategy: throw arbitrary byte slices (0..=256 bytes) at decode.
// Invalid frames are silently skipped - we only assert on values that decoded
// successfully. This catches encoder/decoder asymmetry without needing to
// generate structurally valid UDS frames.

use ace_core::codec::{FrameRead, FrameWrite};
use ace_uds::message::*;
use bytes::BytesMut;
use proptest::prelude::*;

// region: Helpers

/// Encode a value into a fresh BytesMut and return it.
fn encode<T: FrameWrite>(value: &T) -> BytesMut
where
    T::Error: std::fmt::Debug,
{
    let mut buf = BytesMut::new();
    value
        .encode(&mut buf)
        .expect("encode should not fail on a successfully decoded value");
    buf
}

/// Decode a value from a byte slice, returning None if decode fails.
fn try_decode<'a, T: FrameRead<'a>>(bytes: &'a [u8]) -> Option<T> {
    let mut cursor = bytes;
    T::decode(&mut cursor).ok()
}

// endregion: Helpers

// region: Round-trip macro
//
// roundtrip!(test_name, Type) expands to a proptest that:
//   1. Decodes Type from arbitrary bytes - skips if decode fails.
//   2. Encodes the decoded value.
//   3. Decodes again from the encoded bytes.
//   4. Asserts both decoded values are equal.

macro_rules! roundtrip {
    ($name:ident, $ty:ty) => {
        proptest! {
            #[test]
            fn $name(bytes in proptest::collection::vec(any::<u8>(), 0..=256usize)) {
                if let Some(first) = try_decode::<$ty>(&bytes) {
                    let encoded = encode(&first);
                    let second = try_decode::<$ty>(&encoded)
                        .expect("re-decode of encoded value must succeed");
                    prop_assert_eq!(first, second);
                }
            }
        }
    };
}

// endregion: Round-trip macro

// region: DiagnosticSessionControl

roundtrip!(
    rt_diagnostic_session_control_request,
    DiagnosticSessionControlRequest
);
roundtrip!(
    rt_diagnostic_session_control_response,
    DiagnosticSessionControlResponse
);

// endregion: DiagnosticSessionControl

// region: ECUReset

roundtrip!(rt_ecu_reset_request, EcuResetRequest);
roundtrip!(rt_ecu_reset_response, EcuResetResponse);

// endregion: ECUReset

// region: SecurityAccess

roundtrip!(rt_security_access_request, SecurityAccessRequest);
roundtrip!(rt_security_access_response, SecurityAccessResponse);

// endregion: SecurityAccess

// region: CommunicationControl

roundtrip!(
    rt_communication_control_request,
    CommunicationControlRequest
);
roundtrip!(
    rt_communication_control_response,
    CommunicationControlResponse
);

// endregion: CommunicationControl

// region: Authentication

roundtrip!(rt_authentication_request, AuthenticationRequest);
roundtrip!(rt_authentication_response, AuthenticationResponse);

// endregion: Authentication

// region: TesterPresent

roundtrip!(rt_tester_present_request, TesterPresentRequest);
roundtrip!(rt_tester_present_response, TesterPresentResponse);

// endregion: TesterPresent

// region: ControlDTCSetting

roundtrip!(rt_control_dtc_setting_request, ControlDTCSettingRequest);
roundtrip!(rt_control_dtc_setting_response, ControlDTCSettingResponse);

// endregion: ControlDTCSetting

// region: ResponseOnEvent

roundtrip!(rt_response_on_event_request, ResponseOnEventRequest);
roundtrip!(rt_response_on_event_response, ResponseOnEventResponse);

// endregion: ResponseOnEvent

// region: LinkControl

roundtrip!(rt_link_control_request, LinkControlRequest);
roundtrip!(rt_link_control_response, LinkControlResponse);

// endregion: LinkControl

// region: ReadDataByIdentifier

roundtrip!(
    rt_read_data_by_identifier_request,
    ReadDataByIdentifierRequest
);
roundtrip!(
    rt_read_data_by_identifier_response,
    ReadDataByIdentifierResponse
);

// endregion: ReadDataByIdentifier

// region: ReadMemoryByAddress

roundtrip!(
    rt_read_memory_by_address_request,
    ReadMemoryByAddressRequest
);
roundtrip!(
    rt_read_memory_by_address_response,
    ReadMemoryByAddressResponse
);

// endregion: ReadMemoryByAddress

// region: ReadScalingDataByIdentifier

roundtrip!(
    rt_read_scaling_data_by_identifier_request,
    ReadScalingDataByIdentifierRequest
);
roundtrip!(
    rt_read_scaling_data_by_identifier_response,
    ReadScalingDataByIdentifierResponse
);

// endregion: ReadScalingDataByIdentifier

// region: ReadDataByPeriodicIdentifier

roundtrip!(
    rt_read_data_by_periodic_identifier_request,
    ReadDataByPeriodicIdentifierRequest
);
roundtrip!(
    rt_read_data_by_periodic_identifier_response,
    ReadDataByPeriodicIdentifierResponse
);
roundtrip!(
    rt_read_data_by_periodic_identifier_response_data,
    ReadDataByPeriodicIdentifierResponseData
);

// endregion: ReadDataByPeriodicIdentifier

// region: DynamicallyDefineDataIdentifier

roundtrip!(
    rt_dynamically_define_data_identifier_request,
    DynamicallyDefineDataIdentifierRequest
);
roundtrip!(
    rt_dynamically_define_data_identifier_response,
    DynamicallyDefineDataIdentifierResponse
);

// endregion: DynamicallyDefineDataIdentifier

// region: WriteDataByIdentifier

roundtrip!(
    rt_write_data_by_identifier_request,
    WriteDataByIdentifierRequest
);
roundtrip!(
    rt_write_data_by_identifier_response,
    WriteDataByIdentifierResponse
);

// endregion: WriteDataByIdentifier

// region: WriteMemoryByAddress

roundtrip!(
    rt_write_memory_by_address_request,
    WriteMemoryByAddressRequest
);
roundtrip!(
    rt_write_memory_by_address_response,
    WriteMemoryByAddressResponse
);

// endregion: WriteMemoryByAddress

// region: ReadDTCInformation

roundtrip!(rt_read_dtc_information_request, ReadDtcInformationRequest);
roundtrip!(rt_read_dtc_information_response, ReadDTCInformationResponse);

// endregion: ReadDTCInformation

// region: ClearDiagnosticInformation

roundtrip!(
    rt_clear_diagnostic_information_request,
    ClearDiagnosticInformationRequest
);
roundtrip!(
    rt_clear_diagnostic_information_response,
    ClearDiagnosticInformationResponse
);

// endregion: ClearDiagnosticInformation

// region: InputOutputControlByIdentifier

roundtrip!(
    rt_input_output_control_by_identifier_request,
    InputOutputControlByIdentifierRequest
);
roundtrip!(
    rt_input_output_control_by_identifier_response,
    InputOutputControlByIdentifierResponse
);

// endregion: InputOutputControlByIdentifier

// region: RoutineControl

roundtrip!(rt_routine_control_request, RoutineControlRequest);
roundtrip!(rt_routine_control_response, RoutineControlResponse);

// endregion: RoutineControl

// region: RequestDownload

roundtrip!(rt_request_download_request, RequestDownloadRequest);
roundtrip!(rt_request_download_response, RequestDownloadResponse);

// endregion: RequestDownload

// region: RequestUpload

roundtrip!(rt_request_upload_request, RequestUploadRequest);
roundtrip!(rt_request_upload_response, RequestUploadResponse);

// endregion: RequestUpload

// region: TransferData

roundtrip!(rt_transfer_data_request, TransferDataRequest);
roundtrip!(rt_transfer_data_response, TransferDataResponse);

// endregion: TransferData

// region: RequestTransferExit

roundtrip!(rt_request_transfer_exit_request, RequestTransferExitRequest);
roundtrip!(
    rt_request_transfer_exit_response,
    RequestTransferExitResponse
);

// endregion: RequestTransferExit

// region: RequestFileTransfer

roundtrip!(rt_request_file_transfer_request, RequestFileTransferRequest);
roundtrip!(
    rt_request_file_transfer_response,
    RequestFileTransferResponse
);

// endregion: RequestFileTransfer

// region: SecuredDataTransmission

roundtrip!(
    rt_secured_data_transmission_request,
    SecuredDataTransmissionRequest
);
roundtrip!(
    rt_secured_data_transmission_response,
    SecuredDataTransmissionResponse
);

// endregion: SecuredDataTransmission

// region: NegativeResponse

roundtrip!(rt_negative_response, NegativeResponse);

// endregion: NegativeResponse

// region: Entry point - UdsMessage
//
// Tests the full decode path including SID dispatch and Payload construction.
// Uses raw byte slices rather than the inner types since UdsMessage::decode
// reads the SID byte itself.

proptest! {
    #[test]
    fn rt_uds_message(bytes in proptest::collection::vec(any::<u8>(), 0..=256usize)) {
        if let Some(first) = try_decode::<UdsMessage>(&bytes) {
            let encoded = encode(&first);
            let second = try_decode::<UdsMessage>(&encoded)
                .expect("re-decode of encoded UdsMessage must succeed");
            prop_assert_eq!(first, second);
        }
    }
}

// endregion: Entry point - UdsMessage
