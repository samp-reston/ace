use crate::UdsError;
use ace_core::{take_n, FrameRead};
use ace_macros::{FrameCodec, FrameWrite};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum RequestFileTransferRequest<'a> {
    #[frame(id = 0x01)]
    AddFile(AddFileRequest<'a>),
    #[frame(id = 0x02)]
    DeleteFile(DeleteFileRequest<'a>),
    #[frame(id = 0x03)]
    ReplaceFile(ReplaceFileRequest<'a>),
    #[frame(id = 0x04)]
    ReadFile(ReadFileRequest<'a>),
    #[frame(id = 0x05)]
    ReadDir(ReadDirRequest<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct AddFileRequest<'a> {
    pub file_path_and_name_length: [u8; 2],
    pub file_path_and_name: &'a [u8],
    pub data_format_identifier: u8,
    pub file_size_parameter_length: u8,
    pub file_size_uncompressed: &'a [u8],
    pub file_size_compressed: &'a [u8],
}

impl<'a> FrameRead<'a> for AddFileRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let file_path_and_name_length = <[u8; 2]>::decode(buf)?;

        let path_len = u16::from_be_bytes(file_path_and_name_length) as usize;
        let file_path_and_name = take_n(buf, path_len)?;

        let data_format_identifier = u8::decode(buf)?;
        let file_size_parameter_length = u8::decode(buf)?;

        let file_size_len = file_size_parameter_length as usize;
        let file_size_uncompressed = take_n(buf, file_size_len)?;
        let file_size_compressed = take_n(buf, file_size_len)?;

        Ok(Self {
            file_path_and_name_length,
            file_path_and_name,
            data_format_identifier,
            file_size_parameter_length,
            file_size_uncompressed,
            file_size_compressed,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct DeleteFileRequest<'a> {
    pub file_path_and_name_length: [u8; 2],
    pub file_path_and_name: &'a [u8],
}

impl<'a> FrameRead<'a> for DeleteFileRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let file_path_and_name_length = <[u8; 2]>::decode(buf)?;

        let path_len = u16::from_be_bytes(file_path_and_name_length) as usize;
        let file_path_and_name = take_n(buf, path_len)?;

        Ok(Self {
            file_path_and_name_length,
            file_path_and_name,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct ReplaceFileRequest<'a> {
    pub file_path_and_name_length: [u8; 2],
    pub file_path_and_name: &'a [u8],
    pub data_format_identifier: u8,
    pub file_size_parameter_length: u8,
    pub file_size_uncompressed: &'a [u8],
    pub file_size_compressed: &'a [u8],
}

impl<'a> FrameRead<'a> for ReplaceFileRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let file_path_and_name_length = <[u8; 2]>::decode(buf)?;

        let path_len = u16::from_be_bytes(file_path_and_name_length) as usize;
        let file_path_and_name = take_n(buf, path_len)?;

        let data_format_identifier = u8::decode(buf)?;
        let file_size_parameter_length = u8::decode(buf)?;

        let file_size_len = file_size_parameter_length as usize;
        let file_size_uncompressed = take_n(buf, file_size_len)?;
        let file_size_compressed = take_n(buf, file_size_len)?;

        Ok(Self {
            file_path_and_name_length,
            file_path_and_name,
            data_format_identifier,
            file_size_parameter_length,
            file_size_uncompressed,
            file_size_compressed,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct ReadFileRequest<'a> {
    pub file_path_and_name_length: [u8; 2],
    pub file_path_and_name: &'a [u8],
    pub data_format_identifier: u8,
}

impl<'a> FrameRead<'a> for ReadFileRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let file_path_and_name_length = <[u8; 2]>::decode(buf)?;

        let path_len = u16::from_be_bytes(file_path_and_name_length) as usize;
        let file_path_and_name = take_n(buf, path_len)?;

        let data_format_identifier = u8::decode(buf)?;

        Ok(Self {
            file_path_and_name_length,
            file_path_and_name,
            data_format_identifier,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct ReadDirRequest<'a> {
    pub file_path_and_name_length: [u8; 2],
    pub file_path_and_name: &'a [u8],
}

impl<'a> FrameRead<'a> for ReadDirRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let file_path_and_name_length = <[u8; 2]>::decode(buf)?;

        let path_len = u16::from_be_bytes(file_path_and_name_length) as usize;
        let file_path_and_name = take_n(buf, path_len)?;

        Ok(Self {
            file_path_and_name_length,
            file_path_and_name,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum RequestFileTransferResponse<'a> {
    #[frame(id = 0x01)]
    AddFile(AddFileResponse<'a>),
    #[frame(id = 0x02)]
    DeleteFile(DeleteFileResponse),
    #[frame(id = 0x03)]
    ReplaceFile(ReplaceFileResponse<'a>),
    #[frame(id = 0x04)]
    ReadFile(ReadFileResponse<'a>),
    #[frame(id = 0x05)]
    ReadDir(ReadDirResponse<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct AddFileResponse<'a> {
    pub length_format_identifier: u8,
    pub max_number_of_block_length: &'a [u8],
    pub data_format_identifier: u8,
}

impl<'a> FrameRead<'a> for AddFileResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let length_format_identifier = u8::decode(buf)?;

        let block_len = (length_format_identifier >> 4) as usize;
        let max_number_of_block_length = take_n(buf, block_len)?;

        let data_format_identifier = u8::decode(buf)?;

        Ok(Self {
            length_format_identifier,
            max_number_of_block_length,
            data_format_identifier,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DeleteFileResponse {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct ReplaceFileResponse<'a> {
    pub length_format_identifier: u8,
    pub max_number_of_block_length: &'a [u8],
    pub data_format_identifier: u8,
}

impl<'a> FrameRead<'a> for ReplaceFileResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let length_format_identifier = u8::decode(buf)?;

        let block_len = (length_format_identifier >> 4) as usize;
        let max_number_of_block_length = take_n(buf, block_len)?;

        let data_format_identifier = u8::decode(buf)?;

        Ok(Self {
            length_format_identifier,
            max_number_of_block_length,
            data_format_identifier,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct ReadFileResponse<'a> {
    pub length_format_identifier: u8,         // TODO: Evaluate this
    pub max_number_of_block_length: &'a [u8], // TODO: Evaluate this
    pub data_format_identifier: u8,
    pub file_size_or_dir_info_parameter_length: [u8; 2],
    pub file_size_uncompressed_or_dir_info_length: &'a [u8],
    pub file_size_compressed: &'a [u8],
}

impl<'a> FrameRead<'a> for ReadFileResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let length_format_identifier = u8::decode(buf)?; // TODO: Evaluate this
        let block_len = (length_format_identifier >> 4) as usize; // TODO: Evaluate this
        let max_number_of_block_length = take_n(buf, block_len)?; // TODO: Evaluate this
        let data_format_identifier = u8::decode(buf)?;

        // file_size_or_dir_info_parameter_length is a [u8; 2] big-endian length
        // whose value gives the byte length of each of the two following fields
        let file_size_or_dir_info_parameter_length = <[u8; 2]>::decode(buf)?;
        let info_len = u16::from_be_bytes(file_size_or_dir_info_parameter_length) as usize;

        let file_size_uncompressed_or_dir_info_length = take_n(buf, info_len)?;
        let file_size_compressed = take_n(buf, info_len)?;

        Ok(Self {
            length_format_identifier,
            max_number_of_block_length,
            data_format_identifier,
            file_size_or_dir_info_parameter_length,
            file_size_uncompressed_or_dir_info_length,
            file_size_compressed,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct ReadDirResponse<'a> {
    pub length_format_identifier: u8,
    pub max_number_of_block_length: &'a [u8],
    pub data_format_identifier: u8,
    pub file_size_or_dir_info_parameter_length: [u8; 2],
    pub file_size_uncompressed_or_dir_info_length: &'a [u8],
}

impl<'a> FrameRead<'a> for ReadDirResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let length_format_identifier = u8::decode(buf)?;
        let block_len = (length_format_identifier >> 4) as usize;
        let max_number_of_block_length = take_n(buf, block_len)?;
        let data_format_identifier = u8::decode(buf)?;

        let file_size_or_dir_info_parameter_length = <[u8; 2]>::decode(buf)?;
        let info_len = u16::from_be_bytes(file_size_or_dir_info_parameter_length) as usize;

        let file_size_uncompressed_or_dir_info_length = take_n(buf, info_len)?;

        Ok(Self {
            length_format_identifier,
            max_number_of_block_length,
            data_format_identifier,
            file_size_or_dir_info_parameter_length,
            file_size_uncompressed_or_dir_info_length,
        })
    }
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ModeOfOperation {
    #[frame(id = 0x01)]
    AddFile,
    #[frame(id = 0x02)]
    DeleteFile,
    #[frame(id = 0x03)]
    ReplaceFile,
    #[frame(id = 0x04)]
    ReadFile,
    #[frame(id = 0x05)]
    ReadDir,
}
