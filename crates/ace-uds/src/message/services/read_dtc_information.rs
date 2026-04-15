use crate::{message::FunctionalGroup, UdsError};
use ace_core::{DiagError, FrameIter};
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ReadDtcInformationRequest {
    #[frame(id_pat = "0x00 | 0x1B..=0x41 | 0x43..=0x54 | 0x57..=0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    ReportNumberOfDtcByStatusMaskRequest(ReportNumberOfDtcByStatusMaskRequest),
    #[frame(id = 0x02)]
    ReportDtcByStatusMaskRequest(ReportDtcByStatusMaskRequest),
    #[frame(id = 0x03)]
    ReportDtcSnapshotIdentificationRequest(ReportDtcSnapshotIdentificationRequest),
    #[frame(id = 0x04)]
    ReportDtcSnapshotRecordByDtcNumberRequest(ReportDtcSnapshotRecordByDtcNumberRequest),
    #[frame(id = 0x05)]
    ReportDtcStoredDataByRecordNumberRequest(ReportDtcStoredDataByRecordNumberRequest),
    #[frame(id = 0x06)]
    ReportDTCExtDataRecordByDTCNumberRequest(ReportDTCExtDataRecordByDTCNumberRequest),
    #[frame(id = 0x07)]
    ReportNumberOfDTCBySeverityMaskRecordRequest(ReportNumberOfDTCBySeverityMaskRecordRequest),
    #[frame(id = 0x08)]
    ReportDTCSeverityInformationRequest(ReportDTCSeverityInformationRequest),
    #[frame(id = 0x09)]
    ReportSeverityInformationOfDTCRequest(ReportSeverityInformationOfDTCRequest),
    #[frame(id = 0x0A)]
    ReportSupportedDTCRequest(ReportSupportedDTCRequest),
    #[frame(id = 0x0B)]
    ReportFirstTestFailedDTCRequest(ReportFirstTestFailedDTCRequest),
    #[frame(id = 0x0C)]
    ReportFirstConfirmedDTCRequest(ReportFirstConfirmedDTCRequest),
    #[frame(id = 0x0D)]
    ReportMostRecentTestFailedDTCRequest(ReportMostRecentTestFailedDTCRequest),
    #[frame(id = 0x0E)]
    ReportMostRecentConfirmedDTCRequest(ReportMostRecentConfirmedDTCRequest),
    #[frame(id = 0x14)]
    ReportDTCFaultDetectionCounterRequest(ReportDTCFaultDetectionCounterRequest),
    #[frame(id = 0x15)]
    ReportDTCWithPermanentStatusRequest(ReportDTCWithPermanentStatusRequest),
    #[frame(id = 0x16)]
    ReportDTCExtDataRecordByRecordNumberRequest(ReportDTCExtDataRecordByRecordNumberRequest),
    #[frame(id = 0x17)]
    ReportUserDefMemoryDTCByStatusMaskRequest(ReportUserDefMemoryDTCByStatusMaskRequest),
    #[frame(id = 0x18)]
    ReportUserDefMemoryDTCSnapshotRecordByDTCNumberRequest(
        ReportUserDefMemoryDTCSnapshotRecordByDTCNumberRequest,
    ),
    #[frame(id = 0x19)]
    ReportUserDefMemoryDTCExtDataRecordByDTCNumberRequest(
        ReportUserDefMemoryDTCExtDataRecordByDTCNumberRequest,
    ),
    #[frame(id = 0x1A)]
    ReportSupportedDTCExtDataRecordRequest(ReportSupportedDTCExtDataRecordRequest),
    #[frame(id = 0x42)]
    ReportWWHOBDDTCByMaskRecordRequest(ReportWWHOBDDTCByMaskRecordRequest),
    #[frame(id = 0x55)]
    ReportWWHOBDDTCWithPermanentStatusRequest(ReportWWHOBDDTCWithPermanentStatusRequest),
    #[frame(id = 0x56)]
    ReportDTCInformationByDTCReadinessGroupIdentifierRequest(
        ReportDTCInformationByDTCReadinessGroupIdentifierRequest,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DtcStatusMask {
    pub test_failed: bool,
    pub test_failed_this_operational_cycle: bool,
    pub pending_dtc: bool,
    pub confirmed_dtc: bool,
    pub test_not_completed_since_last_clear: bool,
    pub test_failed_since_last_clear: bool,
    pub test_not_completed_this_operation_cycle: bool,
    pub warning_indicator_requested: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDtcByStatusMaskRequest {
    pub dtc_status_mask: DtcStatusMask,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcByStatusMaskRequest {
    pub dtc_status_mask: DtcStatusMask,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcSnapshotIdentificationRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcSnapshotRecordByDtcNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub dtc_snapshot_record_number: DtcSnapshotRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcStoredDataByRecordNumberRequest {
    pub dtc_stored_data_record_number: DtcStoredDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCExtDataRecordByDTCNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDTCBySeverityMaskRecordRequest {
    pub dtc_severity_mask_record: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCSeverityInformationRequest {
    pub dtc_severity_mask_record: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSeverityInformationOfDTCRequest {
    pub dtc_mask_record: [u8; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDTCRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstTestFailedDTCRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstConfirmedDTCRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentTestFailedDTCRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentConfirmedDTCRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCFaultDetectionCounterRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCWithPermanentStatusRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCExtDataRecordByRecordNumberRequest {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDTCByStatusMaskRequest {
    pub dtc_status_mask: DtcStatusMask,
    pub memory_selection: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDTCSnapshotRecordByDTCNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub user_def_dtc_snapshot_record_number: DtcSnapshotRecordNumber,
    pub memory_selection: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDTCExtDataRecordByDTCNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub user_def_dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
    pub memory_selection: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDTCExtDataRecordRequest {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWWHOBDDTCByMaskRecordRequest {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_severity_mask_record: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWWHOBDDTCWithPermanentStatusRequest {
    pub functional_group_identifier: FunctionalGroup,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCInformationByDTCReadinessGroupIdentifierRequest {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_readiness_group_identifier: u8, //TODO: Check if parameterised into enum
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ReportType {
    #[frame(id_pat = "0x00 | 0x1B..=0x41 | 0x43..=0x54 | 0x57..=0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    ReportNumberOfDTCByStatusMask,
    #[frame(id = 0x02)]
    ReportDTCByStatusMask,
    #[frame(id = 0x03)]
    ReportDTCSnapshotIdentification,
    #[frame(id = 0x04)]
    ReportDTCSnapshotRecordByDTCNumber,
    #[frame(id = 0x05)]
    ReportDTCStoredDataByRecordNumber,
    #[frame(id = 0x06)]
    ReportDTCExtDataRecordByDTCNumber,
    #[frame(id = 0x07)]
    ReportNumberOfDTCBySeverityMaskRecord,
    #[frame(id = 0x08)]
    ReportDTCBySeverityMaskRecord,
    #[frame(id = 0x09)]
    ReportSeverityInformationOfDTC,
    #[frame(id = 0x0A)]
    ReportSupportedDTC,
    #[frame(id = 0x0B)]
    ReportFirstTestFailedDTC,
    #[frame(id = 0x0C)]
    ReportFirstConfirmedDTC,
    #[frame(id = 0x0D)]
    ReportMostRecentTestFailedDTC,
    #[frame(id = 0x0E)]
    ReportMostRecentConfirmedDTC,
    #[frame(id = 0x14)]
    ReportDTCFaultDetectionCounter,
    #[frame(id = 0x15)]
    ReportDTCWithPermanentStatus,
    #[frame(id = 0x16)]
    ReportDTCExtDataRecordByRecordNumber,
    #[frame(id = 0x17)]
    ReportUserDefMemoryDTCByStatusMask,
    #[frame(id = 0x18)]
    ReportUserDefMemoryDTCSnapshotRecordByDTCNumber,
    #[frame(id = 0x19)]
    ReportUserDefMemoryDTCExtDataRecordByDTCNumber,
    #[frame(id = 0x1A)]
    ReportSupportedDTCExtDataRecordRequest,
    #[frame(id = 0x42)]
    ReportWWHOBDDTCByMaskRecord,
    #[frame(id = 0x55)]
    ReportWWHOBDDTCWithPermanentStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ReadDTCInformationResponse<'a> {
    #[frame(id = 0x01)]
    ReportNumberOfDTCByStatusMaskResponse(ReportNumberOfDTCByStatusMaskResponse),
    #[frame(id = 0x02)]
    ReportDTCByStatusMaskResponse(ReportDTCByStatusMaskResponse<'a>),
    #[frame(id = 0x03)]
    ReportSnapshotIdentificationResponse(ReportSnapshotIdentificationResponse<'a>),
    #[frame(id = 0x04)]
    ReportDTCSnapshotRecordByDTCNumberResponse(ReportDTCSnapshotRecordByDTCNumberResponse<'a>),
    #[frame(id = 0x05)]
    ReportDTCStoredDataByRecordNumberResponse(ReportDTCStoredDataByRecordNumberResponse<'a>),
    #[frame(id = 0x06)]
    ReportDTCExtDataRecordByDTCNumberResponse(ReportDTCExtDataRecordByDTCNumberResponse<'a>),
    #[frame(id = 0x07)]
    ReportNumberOfDTCBySeverityMaskRecordResponse(ReportNumberOfDTCBySeverityMaskRecordResponse),
    #[frame(id = 0x08)]
    ReportDTCBySeverityMaskRecordResponse(ReportDTCBySeverityMaskRecordResponse<'a>),
    #[frame(id = 0x09)]
    ReportSeverityInformationOfDTCResponse(ReportSeverityInformationOfDTCResponse<'a>),
    #[frame(id = 0x0A)]
    ReportSupportedDTCsResponse(ReportSupportedDTCsResponse<'a>),
    #[frame(id = 0x0B)]
    ReportFirstTestFailedDTCResponse(ReportFirstTestFailedDTCResponse<'a>),
    #[frame(id = 0x0C)]
    ReportFirstConfirmedDTCResponse(ReportFirstConfirmedDTCResponse<'a>),
    #[frame(id = 0x0D)]
    ReportMostRecentTestFailedDTCResponse(ReportMostRecentTestFailedDTCResponse<'a>),
    #[frame(id = 0x0E)]
    ReportMostRecentConfirmedDTCResponse(ReportMostRecentConfirmedDTCResponse<'a>),
    #[frame(id = 0x14)]
    ReportDTCFaultDetectionCounterResponse(ReportDTCFaultDetectionCounterResponse<'a>),
    #[frame(id = 0x15)]
    ReportDTCWithPermanentStatusResponse(ReportDTCWithPermanentStatusResponse<'a>),
    #[frame(id = 0x16)]
    ReportDTCExtDataRecordByRecordNumberResponse(ReportDTCExtDataRecordByRecordNumberResponse<'a>),
    #[frame(id = 0x17)]
    ReportUserDefMemoryDTCByStatusMaskResponse(ReportUserDefMemoryDTCByStatusMaskResponse<'a>),
    #[frame(id = 0x18)]
    ReportUserDefMemoryDTCSnapshotRecordByDTCNumberResponse(
        ReportUserDefMemoryDTCSnapshotRecordByDTCNumberResponse<'a>,
    ),
    #[frame(id = 0x19)]
    ReportUserDefMemoryDTCExtDataRecordByDTCNumberResponse(
        ReportUserDefMemoryDTCExtDataRecordByDTCNumberResponse<'a>,
    ),
    #[frame(id = 0x1A)]
    ReportSupportedDTCExtDataRecordResponse(ReportSupportedDTCExtDataRecordResponse<'a>),
    #[frame(id = 0x42)]
    ReportWWHOBDDTCByMaskRecordResponse(ReportWWHOBDDTCByMaskRecordResponse<'a>),
    #[frame(id = 0x55)]
    ReportWWHOBDDTCWithPermanentStatusResponse(ReportWWHOBDDTCWithPermanentStatusResponse<'a>),
    #[frame(id = 0x56)]
    ReportDTCInformationByReadinessGroupIdentifierResponse(
        ReportDTCInformationByReadinessGroupIdentifierResponse<'a>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDTCByStatusMaskResponse {
    pub dtc_status_availability_mask: u8,
    pub dtc_format_identifier: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDTCBySeverityMaskRecordResponse {
    pub dtc_status_availability_mask: u8,
    pub dtc_format_identifier: u8,
    pub dtc_count: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCByStatusMaskResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDTCsResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstTestFailedDTCResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstConfirmedDTCResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentTestFailedDTCResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentConfirmedDTCResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCWithPermanentStatusResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSnapshotIdentificationResponse<'a> {
    pub dtc_records: FrameIter<'a, DTCRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DTCRecord {
    pub dtc_record: [u8; 3],
    pub dtc_snapshot_record_number: DtcSnapshotRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCSnapshotRecordByDTCNumberResponse<'a> {
    pub dtc_and_status_record: [u8; 4],
    pub dtc_snapshot_records: FrameIter<'a, DTCSnapshotRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DTCSnapshotRecord<'a> {
    pub dtc_snapshot_record_number: DtcSnapshotRecordNumber,
    pub dtc_snapshot_record_number_of_identifiers: u8,
    pub dtc_snapshot_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCStoredDataByRecordNumberResponse<'a> {
    pub dtc_stored_data_records: FrameIter<'a, DTCStoredDataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DTCStoredDataRecord<'a> {
    pub dtc_stored_data_record_number: DtcStoredDataRecordNumber,
    pub dtc_and_status_record: [u8; 4],
    pub dtc_stored_data_record_number_of_identifiers: u8,
    pub dtc_stored_data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCExtDataRecordByDTCNumberResponse<'a> {
    pub dtc_and_status_record: [u8; 4],
    pub dtc_ext_data_records: FrameIter<'a, DTCExtDataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DTCExtDataRecord<'a> {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
    pub dtc_ext_data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCBySeverityMaskRecordResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_severity_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSeverityInformationOfDTCResponse<'a> {
    pub dtc_status_availability_mask: u8,
    pub dtc_and_severity_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCFaultDetectionCounterResponse<'a> {
    pub dtc_fault_detection_counter_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCExtDataRecordByRecordNumberResponse<'a> {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
    pub dtc_and_status_records: FrameIter<'a, ExtDTCAndStatusRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ExtDTCAndStatusRecord<'a> {
    pub dtc_and_status_record: [u8; 4],
    pub dtc_ext_data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDTCByStatusMaskResponse<'a> {
    pub memory_selection: u8,
    pub dtc_status_availability_mask: u8,
    pub dtc_and_status_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDTCSnapshotRecordByDTCNumberResponse<'a> {
    pub memory_selection: u8,
    pub dtc_and_status_record: [u8; 4],
    pub user_def_dtc_snapshot_records: FrameIter<'a, UserDefDTCSnapshotRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct UserDefDTCSnapshotRecord<'a> {
    pub user_def_dtc_snapshot_record_number: DtcSnapshotRecordNumber,
    pub dtc_snapshot_record_number_of_identifiers: u8,
    pub user_def_dtc_snapshot_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDTCExtDataRecordByDTCNumberResponse<'a> {
    pub memory_selection: u8,
    pub dtc_and_status_record: [u8; 4],
    pub dtc_ext_data_record: FrameIter<'a, DTCExtDataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDTCExtDataRecordResponse<'a> {
    pub memory_selection: u8,
    pub dtc_ext_data_record_number: Option<u8>,
    pub dtc_ext_data_records: FrameIter<'a, DTCAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DTCAndStatusRecord {
    pub dtc_and_status_record: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWWHOBDDTCByMaskRecordResponse<'a> {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_status_availability_mask: u8,
    pub dtc_severity_availability_mask: u8,
    pub dtc_format_identifier: u8,
    pub dtc_and_severity_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWWHOBDDTCWithPermanentStatusResponse<'a> {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_status_availability_mask: u8,
    pub dtc_format_identifier: u8,
    pub dtc_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDTCInformationByReadinessGroupIdentifierResponse<'a> {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_status_availability_mask: u8,
    pub dtc_format_identifier: u8,
    pub dtc_readiness_group_identifier: u8,
    pub dtc_and_status_record: &'a [u8],
}

impl<'a> ace_core::codec::FrameRead<'a> for DtcStatusMask {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        Ok(Self {
            test_failed: byte & 0x01 != 0,
            test_failed_this_operational_cycle: byte & 0x02 != 0,
            pending_dtc: byte & 0x04 != 0,
            confirmed_dtc: byte & 0x08 != 0,
            test_not_completed_since_last_clear: byte & 0x10 != 0,
            test_failed_since_last_clear: byte & 0x20 != 0,
            test_not_completed_this_operation_cycle: byte & 0x40 != 0,
            warning_indicator_requested: byte & 0x80 != 0,
        })
    }
}

impl ace_core::codec::FrameWrite for DtcStatusMask {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let byte = (self.test_failed as u8)
            | ((self.test_failed_this_operational_cycle as u8) << 1)
            | ((self.pending_dtc as u8) << 2)
            | ((self.confirmed_dtc as u8) << 3)
            | ((self.test_not_completed_since_last_clear as u8) << 4)
            | ((self.test_failed_since_last_clear as u8) << 5)
            | ((self.test_not_completed_this_operation_cycle as u8) << 6)
            | ((self.warning_indicator_requested as u8) << 7);

        buf.write_bytes(&[byte]).map_err(|e| UdsError::from(e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DtcSnapshotRecordNumber {
    #[frame(id_pat = "0x00 | 0xF0")]
    ReservedForLegislation(u8),
    #[frame(id_pat = "0x01..=0xEF | 0xF0..=0xFE")]
    VehicleManufacturerSpecific(u8),
    #[frame(id = 0xFF)]
    AllDtcSnapshotRecords,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum UserDefDtcSnapshotRecordNumber {
    #[frame(id_pat = "0x00..=0xFE")]
    VehicleManufacturerSpecific(u8),
    #[frame(id = 0xFF)]
    AllUserDefDtcSnapshotRecords,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DtcStoredDataRecordNumber {
    #[frame(id = 0x00)]
    ReservedForLegislation,
    #[frame(id_pat = "0x01..=0xFE")]
    VehicleManufacturerSpecific(u8),
    #[frame(id = 0xFF)]
    AllDtcStoredDataRecords,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DtcExtendedDataRecordNumber {
    #[frame(id_pat = "0x00 | 0xF0..=0xFD")]
    IsoSaeReserved(u8),
    #[frame(id_pat = "0x01..=0x8F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x90..=0x9F")]
    RegulatedEmissionsObdDtcExtDataRecords(u8),
    #[frame(id_pat = "0xA0..=0xEF")]
    RegulatedDtcExtDataREcord(u8),
    #[frame(id = 0xFE)]
    AllRegulatedEmissionsObdDtcExtDataRecords,
    #[frame(id = 0xFF)]
    AllDtcExtDataRecords,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum UserDefDtcExtendedDataRecordNumber {
    #[frame(id_pat = "0x00")]
    IsoSaeReserved(u8),
    #[frame(id_pat = "0x01..=0xFE")]
    VehicleManufacturerSpecific(u8),
    #[frame(id = 0xFF)]
    AllUserDefDtcExtDataRecords,
}
