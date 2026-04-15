use crate::error::DoipError;
use ace_macros::FrameCodec;
use ace_proto::doip::constants::{
    DOIP_COMMON_EID_LEN, DOIP_COMMON_VIN_LEN, DOIP_DIAG_COMMON_SOURCE_LEN,
    DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct VehicleAnnouncementMessage {
    pub vin: [u8; DOIP_COMMON_VIN_LEN],
    pub logical_address: [u8; DOIP_DIAG_COMMON_SOURCE_LEN],
    pub eid: [u8; DOIP_COMMON_EID_LEN],
    pub gid: [u8; DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN],
    pub further_action: ActionCode,
    pub vin_gid_sync: Option<SyncStatus>,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum ActionCode {
    #[frame(id = 0x00)]
    NoFurtherActionRequired,
    #[frame(id_pat = "0x01..=0x0F")]
    Reserved(u8),
    #[frame(id = 0x10)]
    RoutingActivationRequired,
    #[frame(id_pat = "0x11..=0xFF")]
    ReservedForOem(u8),
}

impl From<&ActionCode> for u8 {
    fn from(value: &ActionCode) -> Self {
        match value {
            ActionCode::NoFurtherActionRequired => 0x00,
            ActionCode::Reserved(a) => *a,
            ActionCode::RoutingActivationRequired => 0x10,
            ActionCode::ReservedForOem(a) => *a,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum SyncStatus {
    #[frame(id = 0x00)]
    VinGidSynchronized,
    #[frame(id_pat = "0x01..=0x0F | 0x11..=0xFF")]
    Reserved(u8),
    #[frame(id = 0x10)]
    VinGidNotSynchronised,
}
