use ace_core::{FrameRead, FrameWrite, Writer};
use ace_proto::doip::constants::{DOIP_HEADER_LEN, DOIP_TYPE_OFFSET};

use crate::{
    error::{DoipError, DoipValidationError},
    header::{DoipHeader, PayloadType},
    payload::Payload,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DoipMessage<'a> {
    pub header: DoipHeader,
    pub payload: Payload<'a>,
}

impl<'a> FrameRead<'a> for DoipMessage<'a> {
    type Error = DoipError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        // Payload type is at bytes 2-3 of the DoIP header
        if buf.len() < DOIP_HEADER_LEN {
            return Err(DoipError::Validation(DoipValidationError::FrameTooShort {
                actual: buf.len(),
            }));
        }

        let payload_type_raw =
            u16::from_be_bytes([buf[DOIP_TYPE_OFFSET], buf[DOIP_TYPE_OFFSET + 1]]);
        let payload_type = PayloadType::try_from(payload_type_raw).map_err(|_| {
            DoipError::Validation(DoipValidationError::UnknownPayloadType(payload_type_raw))
        })?;

        let header = DoipHeader::decode(buf)?;
        let payload = Payload::decode(payload_type, buf)?;

        Ok(Self { header, payload })
    }
}

impl FrameWrite for DoipMessage<'_> {
    type Error = DoipError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        self.header.encode(buf)?;
        self.payload.encode(buf)
    }
}
