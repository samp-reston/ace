use crate::error::DoipError;
use ace_macros::FrameCodec;
use ace_uds::message::UdsMessage;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct DiagnosticMessage<'a> {
    pub source_address: LogicalAddress,
    pub target_address: LogicalAddress,
    pub message: UdsMessage<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
#[repr(u16)]
pub enum LogicalAddress {
    #[frame(id = 0x0000)]
    IsoSaeReserved,

    #[frame(id_pat = "0x0001..=0x0DFF | 0x1000..=0x7FFF")]
    VehicleManufacturerSpecific(u16),

    #[frame(id_pat = "0x0E00..=0x0E7F")]
    LegislatedDiagnosticTestEquipment(u16),

    #[frame(id_pat = "0x0E80..=0x0EFF")]
    VehicleManufacturerDiagnosticTestEquipment(u16),

    #[frame(id_pat = "0x0F00..=0x0F7F")]
    OnboardDiagnosticEquipment(u16),

    #[frame(id_pat = "0x0F80..=0x0FFF")]
    OffboardDataCollection(u16),

    #[frame(id_pat = "0x8000..=0xCFFF | 0xF000..=0xFFFF")]
    Reserved(u16),

    #[frame(id_pat = "0xD000..=0xDFFF")]
    ReservedSaeTruckBusCCC(u16),

    #[frame(id_pat = "0xE000..=0xE3FF")]
    WwhObdLogicalAddress(u16),

    #[frame(id_pat = "0xE400..=0xEFFF")]
    VehicleManufacturerFunctionalLogialAddress(u16),
}
