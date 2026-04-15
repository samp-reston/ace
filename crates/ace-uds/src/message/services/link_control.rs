use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum LinkControlRequest {
    #[frame(id = 0x01)]
    VerifyModeTransitionWithFixedParameterRequest(VerifyModeTransitionWithFixedParameterRequest),
    #[frame(id = 0x02)]
    VerifyModeTransitionWithSpecificParameterRequest(
        VerifyModeTransitionWithSpecificParameterRequest,
    ),
    #[frame(id = 0x03)]
    TransitionModeRequst(TransitionModeRequest),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyModeTransitionWithSpecificParameterRequest {
    pub link_record: [u8; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyModeTransitionWithFixedParameterRequest {
    pub link_control_mode_identifier: LinkControlModeIdentifier,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct TransitionModeRequest {
    pub link_control_type: LinkControlType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum LinkControlType {
    #[frame(id_pat = "0x00 | 0x04..=0x3F")]
    IsoSaereserved(u8),
    #[frame(id = 0x01)]
    VerifyModeTransitionWithFixedParameter,
    #[frame(id = 0x02)]
    VerifyModeTransitionWithSpecificParameter,
    #[frame(id = 0x03)]
    TransitionMode,
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct LinkControlResponse {
    pub link_control_type: LinkControlType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum LinkControlModeIdentifier {
    #[frame(id_pat = "0x00 | 0x06..=0x0F | 0x14..=0x1F | 0x21..=0xFF")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    PC9600Baud,
    #[frame(id = 0x02)]
    PC19200Baud,
    #[frame(id = 0x03)]
    PC38400Baud,
    #[frame(id = 0x04)]
    PC57600Baud,
    #[frame(id = 0x05)]
    PC115200Baud,
    #[frame(id = 0x10)]
    Can125000Baud,
    #[frame(id = 0x11)]
    Can250000Baud,
    #[frame(id = 0x12)]
    Can500000Baud,
    #[frame(id = 0x13)]
    Can1000000Baud,
    #[frame(id = 0x20)]
    ProgrammingSetup,
}
