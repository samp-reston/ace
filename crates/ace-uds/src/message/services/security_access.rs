use crate::{UdsError, ValidationError};
use ace_core::{DiagError, FrameWrite};
use ace_macros::{FrameCodec, FrameRead, FrameWrite};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityAccessRequest<'a> {
    RequestSeed(RequestSeed<'a>),
    SendKey(SendKey<'a>),
    IsoSaeReserved,
    IsoSaeReservedRequestSeed(RequestSeed<'a>),
    IsoSaeReservedSendKey(SendKey<'a>),
    PyroTechnicSecurityRequestSeed(RequestSeed<'a>),
    PyroTechnicSecuritySendKey(SendKey<'a>),
    SystemSupplierSpecificRequestSeed(RequestSeed<'a>),
    SystemSupplierSpecificSendKey(SendKey<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameRead, FrameWrite)]
#[frame(error = UdsError)]
pub struct RequestSeed<'a> {
    pub request_seed: u8,
    pub security_access_data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameRead, FrameWrite)]
#[frame(error = UdsError)]
pub struct SendKey<'a> {
    pub send_key: u8,
    pub security_key: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityAccessResponse<'a> {
    KeyResponse(u8),
    SeedResponse(SeedResponse<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct SeedResponse<'a> {
    pub security_access_type: u8,
    pub security_seed: &'a [u8],
}

impl<'a> ace_core::codec::FrameRead<'a> for SecurityAccessResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let access_type = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;

        if access_type % 2 == 1 {
            Ok(Self::SeedResponse(SeedResponse::decode(buf)?))
        } else {
            Ok(Self::KeyResponse(access_type))
        }
    }
}

impl<'a> ace_core::codec::FrameRead<'a> for SecurityAccessRequest<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let access_type = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;

        match access_type {
            0x00 | 0x7F => Ok(Self::IsoSaeReserved),
            0x43..=0x5E if access_type % 2 == 1 => {
                Ok(Self::IsoSaeReservedRequestSeed(RequestSeed::decode(buf)?))
            }
            0x43..=0x5E if access_type % 2 == 0 => {
                Ok(Self::IsoSaeReservedSendKey(SendKey::decode(buf)?))
            }
            0x01..=0x41 if access_type % 2 == 1 => Ok(Self::RequestSeed(RequestSeed::decode(buf)?)),
            0x02..=0x42 if access_type % 2 == 0 => Ok(Self::SendKey(SendKey::decode(buf)?)),
            0x5F => Ok(Self::PyroTechnicSecurityRequestSeed(RequestSeed::decode(
                buf,
            )?)),
            0x60 => Ok(Self::PyroTechnicSecuritySendKey(SendKey::decode(buf)?)),
            0x61..=0x7E if access_type % 2 == 1 => Ok(Self::SystemSupplierSpecificRequestSeed(
                RequestSeed::decode(buf)?,
            )),
            0x61..=0x7E if access_type % 2 == 0 => {
                Ok(Self::SystemSupplierSpecificSendKey(SendKey::decode(buf)?))
            }
            _ => Err(UdsError::Validation(
                ValidationError::InvalidSecurityAccessType(access_type),
            )),
        }
    }
}

impl FrameWrite for SecurityAccessResponse<'_> {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::SeedResponse(inner) => Ok(inner.encode(buf)?),
            Self::KeyResponse(inner) => Ok(inner.encode(buf)?),
        }
    }
}

impl FrameWrite for SecurityAccessRequest<'_> {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::RequestSeed(inner)
            | Self::PyroTechnicSecurityRequestSeed(inner)
            | Self::SystemSupplierSpecificRequestSeed(inner)
            | Self::IsoSaeReservedRequestSeed(inner) => inner.encode(buf),
            Self::SendKey(inner)
            | Self::SystemSupplierSpecificSendKey(inner)
            | Self::PyroTechnicSecuritySendKey(inner)
            | Self::IsoSaeReservedSendKey(inner) => inner.encode(buf),
            Self::IsoSaeReserved => Ok(()),
        }
    }
}
