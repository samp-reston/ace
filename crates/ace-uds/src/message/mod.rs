pub mod payload;
pub mod service;
pub mod services;

use ace_core::{DiagError, FrameRead, FrameWrite};
pub use payload::UdsPayload;
pub use service::ServiceIdentifier;
pub use services::*;

use crate::{constants::MIN_FRAME_LEN, UdsError, ValidationError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UdsMessage<'a> {
    pub sid: Option<ServiceIdentifier>,
    pub payload: UdsPayload<'a>,
}

// region: UdsMessage codec

impl<'a> FrameRead<'a> for UdsMessage<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        if buf.is_empty() {
            return Err(ValidationError::InvalidLength {
                expected: MIN_FRAME_LEN,
                actual: 0,
            }
            .into());
        }

        // Peek at the first byte - if it matches a known SID consume it and
        // dispatch to the appropriate payload decoder. If it does not match,
        // treat the entire buffer as periodic data (no SID consumed).
        let mut sid_buf = *buf;
        match ServiceIdentifier::decode(&mut sid_buf) {
            Ok(sid) => {
                // Advance the real cursor past the SID byte
                *buf = sid_buf;
                let payload = UdsPayload::decode(Some(sid), buf)?;
                Ok(Self {
                    sid: Some(sid),
                    payload,
                })
            }
            Err(_) => {
                // No SID consumed - entire buffer is periodic data
                let payload = UdsPayload::decode(None, buf)?;
                Ok(Self { sid: None, payload })
            }
        }
    }
}

impl FrameWrite for UdsMessage<'_> {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        if let Some(sid) = &self.sid {
            sid.encode(buf).map_err(|e| {
                let d: DiagError = e.into();
                UdsError::from(d)
            })?;
        }
        self.payload.encode(buf)
    }
}

// endregion: UdsMessage codec

pub fn decode_message<'a>(bytes: &'a [u8]) -> Result<UdsMessage<'a>, UdsError> {
    if bytes.is_empty() {
        return Err(ValidationError::InvalidLength {
            expected: MIN_FRAME_LEN,
            actual: 0,
        }
        .into());
    }
    let mut sid_buf = &bytes[..1];
    let sid = ServiceIdentifier::decode(&mut sid_buf)
        .map_err(|_| ValidationError::UnsupportedService(bytes[0]))?;
    let mut payload_buf = &bytes[MIN_FRAME_LEN..];
    let payload = UdsPayload::decode(Some(sid), &mut payload_buf)?;
    Ok(UdsMessage {
        sid: Some(sid),
        payload,
    })
}
