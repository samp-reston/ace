use crate::error::DoipError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct PowerInformationRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct PowerInformationResponse {
    pub power_mode: PowerMode,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum PowerMode {
    #[frame(id = 0x00)]
    NotReady,
    #[frame(id = 0x01)]
    Ready,
    #[frame(id = 0x02)]
    NotSupported,
    #[frame(id_pat = "0x03..=0xFF")]
    Reserved(u8),
}
