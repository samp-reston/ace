// region: Modules

pub mod tokio_codec;

// endregion: Modules

// region: Imports

use crate::error::DoipValidationError;
use crate::ext::DoipFrameExt;
use ace_proto::doip::DoipFrame;
use ace_proto::doip::constants::DOIP_HEADER_LEN;

// endregion: Imports

// region: FrameLimits

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FrameLimits {
    pub max_payload_bytes: usize,
}

// endregion: FrameLimits

// region: Decoding

pub fn decode_frame(buf: &[u8], limits: &FrameLimits) -> DecodeOutcome {
    if buf.len() < DOIP_HEADER_LEN {
        return DecodeOutcome::NeedMoreForHeader;
    }

    let header_frame = DoipFrame::from_slice(&buf[..DOIP_HEADER_LEN]);
    let payload_len = match header_frame.payload_length().try_into() {
        Ok(l) => l,
        Err(_) => return DecodeOutcome::ConversionFailure,
    };

    if payload_len > limits.max_payload_bytes {
        return DecodeOutcome::Invalid(DoipValidationError::FrameTooLarge {
            actual: payload_len,
            limit: limits.max_payload_bytes,
        });
    }

    let total_len = DOIP_HEADER_LEN + payload_len;
    if buf.len() < total_len {
        return DecodeOutcome::NeedMorePayload {
            remaining: total_len - buf.len(),
        };
    }

    let frame = DoipFrame::from_slice(&buf[..total_len]);
    match frame.validate_header() {
        Ok(()) => DecodeOutcome::Frame {
            frame_len: total_len,
        },
        Err(e) => DecodeOutcome::Invalid(e),
    }
}

// endregion: Decoding

// region: DecodeOutcome

/// Result of attempting to decode one frame from a byte buffer.
pub enum DecodeOutcome {
    /// Not enough bytes yet for even a header. No bytes consumed.
    NeedMoreForHeader,

    /// Header is complete; still need `remaining` more payload bytes.
    NeedMorePayload { remaining: usize },

    /// A complete, header-validated frame occupies `buf[..frame_len]`.
    /// Caller is responsible for consuming/splitting that many bytes.
    Frame { frame_len: usize },

    /// Header validation failed, or declared payload exceeds `limit`.
    Invalid(DoipValidationError),

    /// Generic failure mode for u8/16/32/64 into usize.
    ConversionFailure,
}

// endregion: DecodeOutcome
