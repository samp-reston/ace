use crate::error::DoipError;
use ace_macros::FrameCodec;
use ace_proto::doip::constants::{DOIP_DIAG_COMMON_SOURCE_LEN, DOIP_DIAG_COMMON_TARGET_LEN};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct DiagnosticMessageNack {
    pub source_address: [u8; DOIP_DIAG_COMMON_SOURCE_LEN],
    pub target_address: [u8; DOIP_DIAG_COMMON_TARGET_LEN],
    pub nack_code: DiagnosticNackCode,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum DiagnosticNackCode {
    #[frame(id_pat = "0x00..=0x01 | 0x0A..=0xFF")]
    Reserved(u8),
    #[frame(id = 0x02)]
    InvalidSourceAddress,
    #[frame(id = 0x03)]
    UnknownTargetAddress,
    #[frame(id = 0x04)]
    DiagnosticMessageTooLarge,
    #[frame(id = 0x05)]
    OutOfMemory,
    #[frame(id = 0x06)]
    TargetUnreachable,
    #[frame(id = 0x07)]
    UnknownNetwork,
    #[frame(id = 0x08)]
    TransportProtocolError,
}
