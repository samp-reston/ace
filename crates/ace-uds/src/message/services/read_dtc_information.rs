use crate::{UdsError, message::FunctionalGroup};
use ace_core::{CodecError, FrameIter};
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
    ReportDtcExtDataRecordByDtcNumberRequest(ReportDtcExtDataRecordByDtcNumberRequest),
    #[frame(id = 0x07)]
    ReportNumberOfDtcBySeverityMaskRecordRequest(ReportNumberOfDtcBySeverityMaskRecordRequest),
    #[frame(id = 0x08)]
    ReportDtcSeverityInformationRequest(ReportDtcSeverityInformationRequest),
    #[frame(id = 0x09)]
    ReportSeverityInformationOfDtcRequest(ReportSeverityInformationOfDtcRequest),
    #[frame(id = 0x0A)]
    ReportSupportedDtcRequest(ReportSupportedDtcRequest),
    #[frame(id = 0x0B)]
    ReportFirstTestFailedDtcRequest(ReportFirstTestFailedDtcRequest),
    #[frame(id = 0x0C)]
    ReportFirstConfirmedDtcRequest(ReportFirstConfirmedDtcRequest),
    #[frame(id = 0x0D)]
    ReportMostRecentTestFailedDtcRequest(ReportMostRecentTestFailedDtcRequest),
    #[frame(id = 0x0E)]
    ReportMostRecentConfirmedDtcRequest(ReportMostRecentConfirmedDtcRequest),
    #[frame(id = 0x14)]
    ReportDtcFaultDetectionCounterRequest(ReportDtcFaultDetectionCounterRequest),
    #[frame(id = 0x15)]
    ReportDtcWithPermanentStatusRequest(ReportDtcWithPermanentStatusRequest),
    #[frame(id = 0x16)]
    ReportDtcExtDataRecordByRecordNumberRequest(ReportDtcExtDataRecordByRecordNumberRequest),
    #[frame(id = 0x17)]
    ReportUserDefMemoryDtcByStatusMaskRequest(ReportUserDefMemoryDtcByStatusMaskRequest),
    #[frame(id = 0x18)]
    ReportUserDefMemoryDtcSnapshotRecordByDtcNumberRequest(
        ReportUserDefMemoryDtcSnapshotRecordByDtcNumberRequest,
    ),
    #[frame(id = 0x19)]
    ReportUserDefMemoryDtcExtDataRecordByDtcNumberRequest(
        ReportUserDefMemoryDtcExtDataRecordByDtcNumberRequest,
    ),
    #[frame(id = 0x1A)]
    ReportSupportedDtcExtDataRecordRequest(ReportSupportedDtcExtDataRecordRequest),
    #[frame(id = 0x42)]
    ReportWwhObdDtcByMaskRecordRequest(ReportWwhObdDtcByMaskRecordRequest),
    #[frame(id = 0x55)]
    ReportWwhObdDtcWithPermanentStatusRequest(ReportWwhObdDtcWithPermanentStatusRequest),
    #[frame(id = 0x56)]
    ReportDtcInformationByDtcReadinessGroupIdentifierRequest(
        ReportDtcInformationByDtcReadinessGroupIdentifierRequest,
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
pub struct ReportDtcExtDataRecordByDtcNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDtcBySeverityMaskRecordRequest {
    pub dtc_severity_mask_record: DtcSeverityMaskRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcSeverityInformationRequest {
    pub dtc_severity_mask_record: DtcSeverityMaskRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSeverityInformationOfDtcRequest {
    pub dtc_mask_record: [u8; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDtcRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstTestFailedDtcRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstConfirmedDtcRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentTestFailedDtcRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentConfirmedDtcRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcFaultDetectionCounterRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcWithPermanentStatusRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcExtDataRecordByRecordNumberRequest {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDtcByStatusMaskRequest {
    pub dtc_status_mask: DtcStatusMask,
    pub memory_selection: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDtcSnapshotRecordByDtcNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub user_def_dtc_snapshot_record_number: DtcSnapshotRecordNumber,
    pub memory_selection: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDtcExtDataRecordByDtcNumberRequest {
    pub dtc_mask_record: [u8; 3],
    pub user_def_dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
    pub memory_selection: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDtcExtDataRecordRequest {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWwhObdDtcByMaskRecordRequest {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_severity_mask_record: DtcSeverityMaskRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWwhObdDtcWithPermanentStatusRequest {
    pub functional_group_identifier: FunctionalGroup,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcInformationByDtcReadinessGroupIdentifierRequest {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_readiness_group_identifier: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ReportType {
    #[frame(id_pat = "0x00 | 0x1B..=0x41 | 0x43..=0x54 | 0x57..=0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    ReportNumberOfDtcByStatusMask,
    #[frame(id = 0x02)]
    ReportDtcByStatusMask,
    #[frame(id = 0x03)]
    ReportDtcSnapshotIdentification,
    #[frame(id = 0x04)]
    ReportDtcSnapshotRecordByDtcNumber,
    #[frame(id = 0x05)]
    ReportDtcStoredDataByRecordNumber,
    #[frame(id = 0x06)]
    ReportDtcExtDataRecordByDtcNumber,
    #[frame(id = 0x07)]
    ReportNumberOfDtcBySeverityMaskRecord,
    #[frame(id = 0x08)]
    ReportDtcBySeverityMaskRecord,
    #[frame(id = 0x09)]
    ReportSeverityInformationOfDtc,
    #[frame(id = 0x0A)]
    ReportSupportedDtc,
    #[frame(id = 0x0B)]
    ReportFirstTestFailedDtc,
    #[frame(id = 0x0C)]
    ReportFirstConfirmedDtc,
    #[frame(id = 0x0D)]
    ReportMostRecentTestFailedDtc,
    #[frame(id = 0x0E)]
    ReportMostRecentConfirmedDtc,
    #[frame(id = 0x14)]
    ReportDtcFaultDetectionCounter,
    #[frame(id = 0x15)]
    ReportDtcWithPermanentStatus,
    #[frame(id = 0x16)]
    ReportDtcExtDataRecordByRecordNumber,
    #[frame(id = 0x17)]
    ReportUserDefMemoryDtcByStatusMask,
    #[frame(id = 0x18)]
    ReportUserDefMemoryDtcSnapshotRecordByDtcNumber,
    #[frame(id = 0x19)]
    ReportUserDefMemoryDtcExtDataRecordByDtcNumber,
    #[frame(id = 0x1A)]
    ReportSupportedDtcExtDataRecordRequest,
    #[frame(id = 0x42)]
    ReportWwhObdDtcByMaskRecord,
    #[frame(id = 0x55)]
    ReportWwhObdDtcWithPermanentStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ReadDtcInformationResponse<'a> {
    #[frame(id = 0x01)]
    ReportNumberOfDtcByStatusMaskResponse(ReportNumberOfDtcByStatusMaskResponse),
    #[frame(id = 0x02)]
    ReportDtcByStatusMaskResponse(ReportDtcByStatusMaskResponse<'a>),
    #[frame(id = 0x03)]
    ReportSnapshotIdentificationResponse(ReportSnapshotIdentificationResponse<'a>),
    #[frame(id = 0x04)]
    ReportDtcSnapshotRecordByDtcNumberResponse(ReportDtcSnapshotRecordByDtcNumberResponse<'a>),
    #[frame(id = 0x05)]
    ReportDtcStoredDataByRecordNumberResponse(ReportDtcStoredDataByRecordNumberResponse<'a>),
    #[frame(id = 0x06)]
    ReportDtcExtDataRecordByDtcNumberResponse(ReportDtcExtDataRecordByDtcNumberResponse<'a>),
    #[frame(id = 0x07)]
    ReportNumberOfDtcBySeverityMaskRecordResponse(ReportNumberOfDtcBySeverityMaskRecordResponse),
    #[frame(id = 0x08)]
    ReportDtcBySeverityMaskRecordResponse(ReportDtcBySeverityMaskRecordResponse<'a>),
    #[frame(id = 0x09)]
    ReportSeverityInformationOfDtcResponse(ReportSeverityInformationOfDtcResponse<'a>),
    #[frame(id = 0x0A)]
    ReportSupportedDtcsResponse(ReportSupportedDtcsResponse<'a>),
    #[frame(id = 0x0B)]
    ReportFirstTestFailedDtcResponse(ReportFirstTestFailedDtcResponse<'a>),
    #[frame(id = 0x0C)]
    ReportFirstConfirmedDtcResponse(ReportFirstConfirmedDtcResponse<'a>),
    #[frame(id = 0x0D)]
    ReportMostRecentTestFailedDtcResponse(ReportMostRecentTestFailedDtcResponse<'a>),
    #[frame(id = 0x0E)]
    ReportMostRecentConfirmedDtcResponse(ReportMostRecentConfirmedDtcResponse<'a>),
    #[frame(id = 0x14)]
    ReportDtcFaultDetectionCounterResponse(ReportDtcFaultDetectionCounterResponse<'a>),
    #[frame(id = 0x15)]
    ReportDtcWithPermanentStatusResponse(ReportDtcWithPermanentStatusResponse<'a>),
    #[frame(id = 0x16)]
    ReportDtcExtDataRecordByRecordNumberResponse(ReportDtcExtDataRecordByRecordNumberResponse<'a>),
    #[frame(id = 0x17)]
    ReportUserDefMemoryDtcByStatusMaskResponse(ReportUserDefMemoryDtcByStatusMaskResponse<'a>),
    #[frame(id = 0x18)]
    ReportUserDefMemoryDtcSnapshotRecordByDtcNumberResponse(
        ReportUserDefMemoryDtcSnapshotRecordByDtcNumberResponse<'a>,
    ),
    #[frame(id = 0x19)]
    ReportUserDefMemoryDtcExtDataRecordByDtcNumberResponse(
        ReportUserDefMemoryDtcExtDataRecordByDtcNumberResponse<'a>,
    ),
    #[frame(id = 0x1A)]
    ReportSupportedDtcExtDataRecordResponse(ReportSupportedDtcExtDataRecordResponse<'a>),
    #[frame(id = 0x42)]
    ReportWwhObdDtcByMaskRecordResponse(ReportWwhObdDtcByMaskRecordResponse<'a>),
    #[frame(id = 0x55)]
    ReportWwhObdDtcWithPermanentStatusResponse(ReportWwhObdDtcWithPermanentStatusResponse<'a>),
    #[frame(id = 0x56)]
    ReportDtcInformationByReadinessGroupIdentifierResponse(
        ReportDtcInformationByReadinessGroupIdentifierResponse<'a>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDtcByStatusMaskResponse {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_format_identifier: DtcFormatIdentifier,
    pub dtc_count: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportNumberOfDtcBySeverityMaskRecordResponse {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_format_identifier: DtcFormatIdentifier,
    pub dtc_count: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcByStatusMaskResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_record: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcAndStatusRecord {
    pub dtc_record: [u8; 3],
    pub status_of_dtc: DtcStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDtcsResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstTestFailedDtcResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportFirstConfirmedDtcResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentTestFailedDtcResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentConfirmedDtcResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcWithPermanentStatusResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSnapshotIdentificationResponse<'a> {
    pub dtc_records: FrameIter<'a, DtcRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcRecord {
    pub dtc_record: [u8; 3],
    pub dtc_snapshot_record_number: DtcSnapshotRecordNumber,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcSnapshotRecordByDtcNumberResponse<'a> {
    pub dtc_and_status_record: DtcAndStatusRecord,
    pub dtc_snapshot_records: FrameIter<'a, DtcSnapshotRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcSnapshotRecord<'a> {
    pub dtc_snapshot_record_number: DtcSnapshotRecordNumber,
    pub dtc_snapshot_record_number_of_identifiers: u8,
    pub dtc_snapshot_record: FrameIter<'a, DataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcStoredDataByRecordNumberResponse<'a> {
    pub dtc_stored_data_records: FrameIter<'a, DtcStoredDataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcStoredDataRecord<'a> {
    pub dtc_stored_data_record_number: DtcStoredDataRecordNumber,
    pub dtc_and_status_record: DtcAndStatusRecord,
    pub dtc_stored_data_record_number_of_identifiers: u8,
    pub dtc_stored_data_record: FrameIter<'a, DataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcExtDataRecordByDtcNumberResponse<'a> {
    pub dtc_and_status_record: DtcAndStatusRecord,
    pub dtc_ext_data_records: FrameIter<'a, DtcExtDataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcExtDataRecord<'a> {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
    pub dtc_ext_data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcBySeverityMaskRecordResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_severity_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSeverityInformationOfDtcResponse<'a> {
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_severity_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcFaultDetectionCounterResponse<'a> {
    pub dtc_fault_detection_counter_record: FrameIter<'a, DtcFaultDetectionRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcExtDataRecordByRecordNumberResponse<'a> {
    pub dtc_ext_data_record_number: DtcExtendedDataRecordNumber,
    pub dtc_and_status_records: FrameIter<'a, ExtDtcAndStatusRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ExtDtcAndStatusRecord<'a> {
    pub dtc_and_status_record: DtcAndStatusRecord,
    pub dtc_ext_data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDtcByStatusMaskResponse<'a> {
    pub memory_selection: u8,
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_and_status_record: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDtcSnapshotRecordByDtcNumberResponse<'a> {
    pub memory_selection: u8,
    pub dtc_and_status_record: DtcAndStatusRecord,
    pub user_def_dtc_snapshot_records: FrameIter<'a, UserDefDTCSnapshotRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct UserDefDTCSnapshotRecord<'a> {
    pub user_def_dtc_snapshot_record_number: DtcSnapshotRecordNumber,
    pub dtc_snapshot_record_number_of_identifiers: u8,
    pub user_def_dtc_snapshot_record: FrameIter<'a, DataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportUserDefMemoryDtcExtDataRecordByDtcNumberResponse<'a> {
    pub memory_selection: u8,
    pub dtc_and_status_record: DtcAndStatusRecord,
    pub dtc_ext_data_record: FrameIter<'a, DtcExtDataRecord<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportSupportedDtcExtDataRecordResponse<'a> {
    pub memory_selection: u8,
    pub dtc_ext_data_record_number: Option<u8>,
    pub dtc_ext_data_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWwhObdDtcByMaskRecordResponse<'a> {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_severity_availability_mask: u8,
    pub dtc_format_identifier: WwhObdFormatIdentifier,
    pub dtc_and_severity_records: FrameIter<'a, DtcAndSeverityRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportWwhObdDtcWithPermanentStatusResponse<'a> {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_format_identifier: WwhObdFormatIdentifier,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcInformationByReadinessGroupIdentifierResponse<'a> {
    pub functional_group_identifier: FunctionalGroup,
    pub dtc_status_availability_mask: DtcStatus,
    pub dtc_format_identifier: u8,
    pub dtc_readiness_group_identifier: u8,
    pub dtc_and_status_records: FrameIter<'a, DtcAndStatusRecord>,
}

impl<'a> ace_core::codec::FrameRead<'a> for DtcStatusMask {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(CodecError::LengthMismatch {
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

        buf.write_bytes(&[byte]).map_err(UdsError::from)
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcSeverityMaskRecord {
    pub dtc_severity_mask: DtcSeverity,
    pub dtc_status_mask: DtcStatusMask,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DtcSeverity {
    pub dtc_serverity_information: DtcSeverityInformation,
    pub dtc_class_information: DtcClassInformation,
}

impl<'a> ace_core::codec::FrameRead<'a> for DtcSeverity {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(CodecError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        Ok(Self {
            dtc_serverity_information: DtcSeverityInformation {
                maintenance_only: byte & 0x20 != 0,
                check_at_next_halt: byte & 0x40 != 0,
                check_immediately: byte & 0x80 != 0,
            },
            dtc_class_information: DtcClassInformation {
                dtc_class_0: byte & 0x01 != 0,
                dtc_class_1: byte & 0x02 != 0,
                dtc_class_2: byte & 0x04 != 0,
                dtc_class_3: byte & 0x08 != 0,
                dtc_class_4: byte & 0x10 != 0,
            },
        })
    }
}

impl ace_core::codec::FrameWrite for DtcSeverity {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let sev = &self.dtc_serverity_information;
        let cls = &self.dtc_class_information;

        let byte = (cls.dtc_class_0 as u8)
            | ((cls.dtc_class_1 as u8) << 1)
            | ((cls.dtc_class_2 as u8) << 2)
            | ((cls.dtc_class_3 as u8) << 3)
            | ((cls.dtc_class_4 as u8) << 4)
            | ((sev.maintenance_only as u8) << 5)
            | ((sev.check_at_next_halt as u8) << 6)
            | ((sev.check_immediately as u8) << 7);

        buf.write_bytes(&[byte]).map_err(UdsError::from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DtcSeverityInformation {
    pub maintenance_only: bool,
    pub check_at_next_halt: bool,
    pub check_immediately: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DtcClassInformation {
    pub dtc_class_0: bool,
    pub dtc_class_1: bool,
    pub dtc_class_2: bool,
    pub dtc_class_3: bool,
    pub dtc_class_4: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DtcStatus {
    pub test_failed: bool,
    pub test_failed_this_operation_cycle: bool,
    pub pending_dtc: bool,
    pub confirmed_dtc: bool,
    pub test_not_completed_since_last_clear: bool,
    pub test_failed_since_last_clear: bool,
    pub test_not_completed_this_operation_cycle: bool,
    pub warning_indicator_requested: bool,
}

impl<'a> ace_core::codec::FrameRead<'a> for DtcStatus {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(CodecError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        Ok(Self {
            test_failed: byte & 0x01 != 0,
            test_failed_this_operation_cycle: byte & 0x02 != 0,
            pending_dtc: byte & 0x04 != 0,
            confirmed_dtc: byte & 0x08 != 0,
            test_not_completed_since_last_clear: byte & 0x10 != 0,
            test_failed_since_last_clear: byte & 0x20 != 0,
            test_not_completed_this_operation_cycle: byte & 0x40 != 0,
            warning_indicator_requested: byte & 0x80 != 0,
        })
    }
}

impl ace_core::codec::FrameWrite for DtcStatus {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let byte = (self.test_failed as u8)
            | ((self.test_failed_this_operation_cycle as u8) << 1)
            | ((self.pending_dtc as u8) << 2)
            | ((self.confirmed_dtc as u8) << 3)
            | ((self.test_not_completed_since_last_clear as u8) << 4)
            | ((self.test_failed_since_last_clear as u8) << 5)
            | ((self.test_not_completed_this_operation_cycle as u8) << 6)
            | ((self.warning_indicator_requested as u8) << 7);

        buf.write_bytes(&[byte]).map_err(UdsError::from)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DtcFormatIdentifier {
    #[frame(id = 0x00)]
    SaeJ2012DaDtcFormat00,
    #[frame(id = 0x01)]
    Iso142291DtcFormat,
    #[frame(id = 0x02)]
    SaeJ193973DtcFormat,
    #[frame(id = 0x03)]
    Iso119924DtcFormat,
    #[frame(id = 0x04)]
    SaeJ2012DaDtcFormat04,
    #[frame(id_pat = "0x05..=0xFF")]
    IsoSaeReserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcAndSeverityRecord {
    pub dtc_severity: DtcSeverity,
    pub dtc_functional_unit: u8,
    pub dtc_record: [u8; 3],
    pub status_of_dtc: DtcStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DtcFaultDetectionRecord {
    pub dtc_record: [u8; 3],
    pub counter: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DataRecord<'a> {
    pub data_identifier: [u8; 2],
    pub data: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum WwhObdFormatIdentifier {
    #[frame(id = 0x02)]
    SaeJ2012DaDtcFormat04,
    #[frame(id = 0x04)]
    SaeJ193973DtcFormat,
}
