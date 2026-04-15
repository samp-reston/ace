use crate::{message::DataIdentifier, UdsError};
use ace_core::{FrameIter, FrameRead};
use ace_macros::{FrameCodec, FrameWrite};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DynamicallyDefineDataIdentifierRequest<'a> {
    #[frame(id_pat = "0x00 | 0x04..=0x7F")]
    IsoSaeReserved(u8),
    #[frame(id = 0x01)]
    DefineByIdentifierRequest(DefineByIdentifierRequest<'a>),
    #[frame(id = 0x02)]
    DefineByMemoryAddressRequest(DefineByMemoryAddressRequest<'a>),
    #[frame(id = 0x03)]
    ClearDynamicallyDefinedDataIdentifier(ClearDynamicallyDefinedDataIdentifier),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DefineByIdentifierRequest<'a> {
    pub dynamically_defined_data_identifier: DataIdentifier,
    pub source_data: FrameIter<'a, SourceData>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct DefineByMemoryAddressRequest<'a> {
    pub dynamically_defined_data_identifier: DataIdentifier,
    pub address_and_length_format_identifier: u8,
    pub memory_data: FrameIter<'a, MemoryData<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ClearDynamicallyDefinedDataIdentifier {
    pub dynamically_defined_data_identifier: DataIdentifier,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct MemoryData<'a> {
    pub memory_address: &'a [u8],
    pub memory_size: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum DefinitionType {
    #[frame(id = 0x01)]
    DefineByIdentifier,
    #[frame(id = 0x02)]
    DefineByMemoryAddress,
    #[frame(id = 0x03)]
    ClearDynamicallyDefinedDataIdentifier,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct SourceData {
    pub source_data_identifier: DataIdentifier,
    pub position_in_source_data_record: u8,
    pub memory_size: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DynamicallyDefineDataIdentifierResponse {
    pub definition_type: DefinitionType,
    pub dynamically_defined_data_identifier: DataIdentifier,
}

impl<'a> FrameRead<'a> for DefineByMemoryAddressRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let dynamically_defined_data_identifier = DataIdentifier::decode(buf)?;
        let address_and_length_format_identifier = u8::decode(buf)?;

        let memory_address_length = (address_and_length_format_identifier & 0x0F) as usize;
        let memory_size_length = (address_and_length_format_identifier >> 4) as usize;
        let stride = memory_address_length + memory_size_length;

        // Truncate to the largest multiple of stride that fits in the buffer,
        // discarding any trailing bytes that don't form a complete record.
        let memory_data_len = if stride == 0 {
            0
        } else {
            (buf.len() / stride) * stride
        };

        let memory_data = FrameIter::new(&buf[..memory_data_len]);
        *buf = &buf[memory_data_len..];

        Ok(Self {
            dynamically_defined_data_identifier,
            address_and_length_format_identifier,
            memory_data,
        })
    }
}
