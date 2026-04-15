use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DiagnosticSessionControlRequest {
    pub diagnostic_session_type: DiagnosticSessionType,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DiagnosticSessionType {
    #[frame(id_pat = "0x00 | 0x05..=0x3F | 0x7F")]
    ISOSAEReserved(u8),
    #[frame(id = 0x01)]
    DefaultSession = 0x01,
    #[frame(id = 0x02)]
    ProgrammingSession = 0x02,
    #[frame(id = 0x03)]
    ExtendedDiagnosticSession = 0x03,
    #[frame(id = 0x04)]
    SafetySystemDiagnosticSession = 0x04,
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DiagnosticSessionControlResponse {
    pub diagnostic_session_type: DiagnosticSessionType,
    pub session_parameter_record: SessionParameterRecord,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct SessionParameterRecord {
    pub p2_server_max: u16,
    pub p2_star_server_max: u16,
}
