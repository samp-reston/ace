use crate::{message::DataIdentifier, UdsError};
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct WriteDataByIdentifierRequest<'a> {
    pub data_identifier: DataIdentifier,
    pub data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct WriteDataByIdentifierResponse {
    pub data_identifier: DataIdentifier,
}
