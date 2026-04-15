use crate::message::DiagError;
use crate::message::{FrameRead, FrameWrite};
use crate::UdsError;
use ace_macros::{FrameCodec, FrameWrite};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct SecuredDataTransmissionRequest<'a> {
    pub administrative_parameter: [u8; 2],
    pub signature_encryption_calculation: u8,
    pub signature_length: [u8; 2],
    pub anti_replay_counter: [u8; 2],
    pub internal_message_service_request_id: u8,
    pub server_specific_parameters: &'a [u8],
    pub signature_mac_byte: &'a [u8],
}

impl<'a> FrameRead<'a> for SecuredDataTransmissionRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let administrative_parameter = <[u8; 2]>::decode(buf)?;
        let signature_encryption_calculation = u8::decode(buf)?;
        let signature_length = <[u8; 2]>::decode(buf)?;
        let sig_len = u16::from_be_bytes(signature_length) as usize;
        let anti_replay_counter = <[u8; 2]>::decode(buf)?;
        let internal_message_service_request_id = u8::decode(buf)?;

        let remaining = *buf;

        let (server_specific_parameters, signature_mac_byte) = if remaining.len() == sig_len {
            *buf = &[];
            (&[][..], remaining)
        } else if remaining.len() < sig_len {
            return Err(UdsError::from(DiagError::LengthMismatch {
                expected: sig_len,
                actual: remaining.len(),
            }));
        } else {
            let split_at = remaining.len() - sig_len;
            let (params, sig) = remaining.split_at(split_at);
            *buf = &[];
            (params, sig)
        };

        Ok(Self {
            administrative_parameter,
            signature_encryption_calculation,
            signature_length,
            anti_replay_counter,
            internal_message_service_request_id,
            server_specific_parameters,
            signature_mac_byte,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuredDataTransmissionResponse<'a> {
    PositiveInternalMessageResponse(PositiveInternalMessageResponse<'a>),
    NegativeInternalMessageResponse(NegativeInternalMessageResponse<'a>),
}

impl<'a> FrameRead<'a> for SecuredDataTransmissionResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        // Byte 7 is the internal message SID - 0x7F means negative response.
        // Minimum 8 bytes required to peek at it.
        if buf.len() <= 7 {
            return Err(UdsError::from(DiagError::LengthMismatch {
                expected: 8,
                actual: buf.len(),
            }));
        };

        match buf[7] {
            0x7F => Ok(Self::NegativeInternalMessageResponse(
                NegativeInternalMessageResponse::decode(buf)?,
            )),
            _ => Ok(Self::PositiveInternalMessageResponse(
                PositiveInternalMessageResponse::decode(buf)?,
            )),
        }
    }
}

impl FrameWrite for SecuredDataTransmissionResponse<'_> {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::PositiveInternalMessageResponse(inner) => inner.encode(buf),
            Self::NegativeInternalMessageResponse(inner) => inner.encode(buf),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameWrite)]
#[frame(error = UdsError)]
pub struct PositiveInternalMessageResponse<'a> {
    pub administrative_parameter: [u8; 2],
    pub signature_encryption_calculation: u8,
    pub signature_length: [u8; 2],
    pub anti_replay_counter: [u8; 2],
    pub internal_message_service_response_id: u8,
    pub response_specific_parameters: &'a [u8],
    pub signature_mac_byte: &'a [u8],
}

impl<'a> FrameRead<'a> for PositiveInternalMessageResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let administrative_parameter = <[u8; 2]>::decode(buf)?;
        let signature_encryption_calculation = u8::decode(buf)?;
        let signature_length = <[u8; 2]>::decode(buf)?;
        let sig_len = u16::from_be_bytes(signature_length) as usize;
        let anti_replay_counter = <[u8; 2]>::decode(buf)?;
        let internal_message_service_response_id = u8::decode(buf)?;

        let remaining = *buf;

        let (response_specific_parameters, signature_mac_byte) = if remaining.len() == sig_len {
            *buf = &[];
            (&[][..], remaining)
        } else if remaining.len() < sig_len {
            return Err(UdsError::from(DiagError::LengthMismatch {
                expected: sig_len,
                actual: remaining.len(),
            }));
        } else {
            let split_at = remaining.len() - sig_len;
            let (params, sig) = remaining.split_at(split_at);
            *buf = &[];
            (params, sig)
        };

        Ok(Self {
            administrative_parameter,
            signature_encryption_calculation,
            signature_length,
            anti_replay_counter,
            internal_message_service_response_id,
            response_specific_parameters,
            signature_mac_byte,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct NegativeInternalMessageResponse<'a> {
    pub administrative_parameter: [u8; 2],
    pub signature_encryption_calculation: u8,
    pub signature_length: [u8; 2],
    pub anti_replay_counter: [u8; 2],
    pub internal_message_negative_response_sid: u8,
    pub internal_message_service_request_id: u8,
    pub internal_message_response_code: u8,
    pub signature_mac_byte: &'a [u8],
}
