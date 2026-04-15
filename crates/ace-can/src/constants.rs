// region: Common

pub const ID_OFFSET: usize = 0;
pub const DLC_OFFSET: usize = 4;
pub const DATA_OFFSET: usize = 5;

pub const EFF_FLAG: u32 = 0x8000_0000;
pub const RTR_FLAG: u32 = 0x4000_0000;
pub const ERR_FLAG: u32 = 0x2000_0000;
pub const STD_ID_MASK: u32 = 0x0000_07FF;
pub const EXT_ID_MASK: u32 = 0x1FFF_FFFF;

// endregion: Common

// region: Classic

pub const CLASSIC_CAN_MAX_DLC: u8 = 8;
pub const CLASSIC_CAN_MIN_LEN: usize = DATA_OFFSET; // 5 bytes - ID word + DLC

// endregion: Classic

// region: FD

pub const FLAGS_OFFSET: usize = 4;
pub const RRS_FLAG: u32 = 0x4000_0000;

pub const EDL_FLAG: u8 = 0b0000_0100;
pub const BRS_FLAG: u8 = 0b0000_0010;
pub const ESI_FLAG: u8 = 0b0000_0001;

pub const CAN_FD_MAX_DLC: u8 = 15;
pub const CAN_FD_MAX_DATA: usize = 64;
pub const CAN_FD_MIN_LEN: usize = DATA_OFFSET;

// endregion: FD

// region: PCI

pub const SF_TYPE: u8 = 0x00;
pub const FF_TYPE: u8 = 0x10;
pub const CF_TYPE: u8 = 0x20;
pub const FC_TYPE: u8 = 0x30;
pub const TYPE_MASK: u8 = 0xF0;
pub const NIBBLE_MASK: u8 = 0x0F;

// Classic CAN FF max length expressible in 12-bit field
pub const FF_MAX_LEN_CLASSIC: u32 = 0xFFF;

// CAN FD escape sequence - FF length field is 0x000, followed by 4-byte length
pub const FF_ESCAPE: u16 = 0x0000;

// endregion: PCI
