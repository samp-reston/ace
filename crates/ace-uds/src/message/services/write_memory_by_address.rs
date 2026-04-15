use crate::UdsError;
use ace_core::{take_n, FrameRead};
use ace_macros::FrameWrite;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct WriteMemoryByAddressRequest<'a> {
    pub address_and_length_format_identifier: u8,
    pub memory_address: &'a [u8],
    pub memory_size: &'a [u8],
    pub data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct WriteMemoryByAddressResponse<'a> {
    pub address_and_length_format_identifier: u8,
    pub memory_address: &'a [u8],
    pub memory_size: &'a [u8],
}

impl<'a> FrameRead<'a> for WriteMemoryByAddressRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let address_and_length_format_identifier = u8::decode(buf)?;

        let memory_address_length = (address_and_length_format_identifier & 0x0F) as usize;
        let memory_size_length = (address_and_length_format_identifier >> 4) as usize;

        let memory_address = take_n(buf, memory_address_length)?;
        let memory_size = take_n(buf, memory_size_length)?;

        let data_record = *buf;
        *buf = &buf[buf.len()..];

        Ok(Self {
            address_and_length_format_identifier,
            memory_address,
            memory_size,
            data_record,
        })
    }
}

impl<'a> FrameRead<'a> for WriteMemoryByAddressResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let address_and_length_format_identifier = u8::decode(buf)?;

        let memory_address_length = (address_and_length_format_identifier & 0x0F) as usize;
        let memory_size_length = (address_and_length_format_identifier >> 4) as usize;

        let memory_address = take_n(buf, memory_address_length)?;
        let memory_size = take_n(buf, memory_size_length)?;

        Ok(Self {
            address_and_length_format_identifier,
            memory_address,
            memory_size,
        })
    }
}
