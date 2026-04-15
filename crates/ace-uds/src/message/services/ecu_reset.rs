use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct EcuResetRequest {
    pub reset_type: ResetType,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ResetType {
    #[frame(id_pat = "0x00 | 0x06..=0x3F | 0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    HardReset,
    #[frame(id = 0x02)]
    KeyOffOnReset,
    #[frame(id = 0x03)]
    SoftReset,
    #[frame(id = 0x04)]
    EnableRapidPowerShutDown,
    #[frame(id = 0x05)]
    DisableRapidPowerShutDown,
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
// #[frame(error = UdsError)]
// pub struct EcuResetResponse {
//     pub reset_type: ResetType,
//     pub power_down_time: Option<u8>,
// }

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum EcuResetResponse {
    #[frame(id_pat = "0x00 | 0x06..=0x3F | 0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    HardReset,
    #[frame(id = 0x02)]
    KeyOffOnReset,
    #[frame(id = 0x03)]
    SoftReset,
    #[frame(id = "0x04")]
    EnableRapidPowerShutDown(EnableRapidPowerShutDown),
    #[frame(id = 0x05)]
    DisableRapidPowerShutDown,
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct EnableRapidPowerShutDown {
    pub power_down_time: PowerDownTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum PowerDownTime {
    #[frame(id_pat = "0x00..=0xFE")]
    Valid(u8),
    #[frame(id = "0xFF")]
    FailureOrTimeNotAvailable,
}
