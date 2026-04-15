use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct TesterPresentRequest {
    pub zero_sub_function: ZeroSubFunction,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct TesterPresentResponse {
    pub zero_sub_function: ZeroSubFunction,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ZeroSubFunction {
    #[frame(id = 0x00)]
    ZeroSubFunction,
    #[frame(id_pat = "0x01..=0x7F")]
    IsoSaeReserved(u8),
}
