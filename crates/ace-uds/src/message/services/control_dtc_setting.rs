use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ControlDTCSettingRequest<'a> {
    pub dtc_setting_type: DtcSettingType,
    pub dtc_setting_control_option_record: &'a [u8],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DtcSettingType {
    #[frame(id_pat = "0x00 | 0x03..=0x3F | 0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    On,
    #[frame(id = 0x02)]
    Off,
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ControlDTCSettingResponse {
    pub dtc_setting_type: DtcSettingType,
}
