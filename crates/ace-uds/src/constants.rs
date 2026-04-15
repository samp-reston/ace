// region: Frame Offsets

/// Byte offset of the Service Identifier within a UDS frame.
pub const SID_OFFSET: usize = 0;

/// Byte offset of the sub-function byte within a UDS frame.
/// Only valid for services where `ServiceIdentifier::has_sub_function()` is true.
pub const SUB_FUNCTION_OFFSET: usize = 1;

/// Byte offset of the requested SID within a negative response frame.
/// Negative response format: [0x7F, RequestedSID, NRC]
pub const NEGATIVE_RESPONSE_REQUESTED_SID_OFFSET: usize = 1;

/// Byte offset of the NRC byte within a negative response frame.
/// Negative response format: [0x7F, RequestedSID, NRC]
pub const NEGATIVE_RESPONSE_NRC_OFFSET: usize = 2;

// endregion: Frame Offsets

// region: Frame Lengths

/// Minimum length of any valid UDS frame - must contain at least a SID byte.
pub const MIN_FRAME_LEN: usize = 1;

/// Minimum length of a UDS frame carrying a sub-function byte.
pub const MIN_SUB_FUNCTION_FRAME_LEN: usize = 2;

/// Minimum length of a negative response frame.
/// Must contain: SID (0x7F) + RequestedSID + NRC
pub const MIN_NEGATIVE_RESPONSE_LEN: usize = 3;

// endregion: Frame Lengths

// region: Sub-Function Masks

/// Bit mask for the suppress positive response bit in the sub-function byte.
/// Per ISO 14229, bit 7 of the sub-function byte controls response suppression.
pub const SUPPRESS_POSITIVE_RESPONSE_MASK: u8 = 0x80;

/// Bit mask for extracting the sub-function value without the suppress bit.
pub const SUB_FUNCTION_VALUE_MASK: u8 = 0x7F;

// endregion: Sub-Function Masks
