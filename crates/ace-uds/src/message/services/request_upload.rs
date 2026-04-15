use ace_core::{take_n, FrameRead};
use ace_macros::FrameWrite;

use crate::UdsError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct RequestUploadRequest<'a> {
    pub data_format_identifier: u8,
    pub address_and_length_format_identifier: u8,
    pub memory_address: &'a [u8],
    pub memory_size: &'a [u8],
}

impl<'a> FrameRead<'a> for RequestUploadRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let data_format_identifier = u8::decode(buf)?;
        let address_and_length_format_identifier = u8::decode(buf)?;

        let memory_address_length = (address_and_length_format_identifier & 0x0F) as usize;
        let memory_size_length = (address_and_length_format_identifier >> 4) as usize;

        let memory_address = take_n(buf, memory_address_length)?;
        let memory_size = take_n(buf, memory_size_length)?;

        Ok(Self {
            data_format_identifier,
            address_and_length_format_identifier,
            memory_address,
            memory_size,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct RequestUploadResponse<'a> {
    pub length_format_identifier: u8,
    pub max_number_of_block_length: &'a [u8],
}

impl<'a> FrameRead<'a> for RequestUploadResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let length_format_identifier = u8::decode(buf)?;
        let max_number_of_block_length_length = (length_format_identifier >> 4) as usize;
        let max_number_of_block_length = take_n(buf, max_number_of_block_length_length)?;

        Ok(Self {
            length_format_identifier,
            max_number_of_block_length,
        })
    }
}
