use ace_macros::FrameCodec;
use ace_proto::doip::constants::{DOIP_DIAG_COMMON_SOURCE_LEN, DOIP_DIAG_COMMON_TARGET_LEN};

use crate::error::DoipError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct DiagnosticMessageAck<'a> {
    pub source_address: [u8; DOIP_DIAG_COMMON_SOURCE_LEN],
    pub target_address: [u8; DOIP_DIAG_COMMON_TARGET_LEN],
    pub ack_code: DiagnosticAckCode,
    pub data: &'a [u8],
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum DiagnosticAckCode {
    #[frame(id = 0x00)]
    Acknowledged,
    #[frame(id_pat = "0x01..=0xFF")]
    Reserved(u8),
}
