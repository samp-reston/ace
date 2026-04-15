use crate::error::DoipError;
use ace_macros::FrameCodec;
use ace_proto::doip::constants::DOIP_DIAG_COMMON_SOURCE_LEN;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct AliveCheckRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct AliveCheckResponse {
    pub source_address: [u8; DOIP_DIAG_COMMON_SOURCE_LEN],
}
