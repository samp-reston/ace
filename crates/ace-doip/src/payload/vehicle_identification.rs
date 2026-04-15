use ace_macros::FrameCodec;
use ace_proto::doip::constants::{DOIP_COMMON_EID_LEN, DOIP_COMMON_VIN_LEN};

use crate::error::DoipError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct VehicleIdentificationRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct VehicleIdentificationRequestEid {
    pub eid: [u8; DOIP_COMMON_EID_LEN],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct VehicleIdentificationRequestVin {
    pub vin: [u8; DOIP_COMMON_VIN_LEN],
}
