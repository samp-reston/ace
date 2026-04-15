use crate::error::DoipError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct GenericNack {
    pub nack_code: NackCode,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum NackCode {
    #[frame(id = 0x00)]
    IncorrectPatternFormat,
    #[frame(id = 0x01)]
    UnknownPayloadType,
    #[frame(id = 0x02)]
    MessageTooLarge,
    #[frame(id = 0x03)]
    OutOfMemory,
    #[frame(id = 0x04)]
    InvalidPayloadLength,
    #[frame(id_pat = "0x05..=0xFF")]
    Reserved(u8),
}
