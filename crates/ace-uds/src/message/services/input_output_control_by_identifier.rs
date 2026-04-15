use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct InputOutputControlByIdentifierRequest<'a> {
    pub data_identifier: [u8; 2],
    pub control_option_record: &'a [u8],
    pub control_enable_mask_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct InputOutputControlByIdentifierResponse<'a> {
    data_identifier: [u8; 2],
    control_status_record: &'a [u8],
}
