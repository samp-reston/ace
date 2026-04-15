pub mod alive_check;
pub mod diagnostic_message;
pub mod diagnostic_message_ack;
pub mod diagnostic_message_nack;
pub mod entity_status;
pub mod generic_nack;
pub mod power_information;
pub mod routing_activation;
pub mod vehicle_announcement_message;
pub mod vehicle_identification;

use ace_core::{FrameRead, FrameWrite};
pub use alive_check::*;
pub use diagnostic_message::*;
pub use diagnostic_message_ack::*;
pub use diagnostic_message_nack::*;
pub use entity_status::*;
pub use generic_nack::*;
pub use power_information::*;
pub use routing_activation::*;
pub use vehicle_announcement_message::*;
pub use vehicle_identification::*;

use crate::{error::DoipError, header::PayloadType};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Payload<'a> {
    GenericNack(GenericNack),
    VehicleIdentificationRequest(VehicleIdentificationRequest),
    VehicleIdentificationRequestEid(VehicleIdentificationRequestEid),
    VehicleIdentificationRequestVin(VehicleIdentificationRequestVin),
    VehicleAnnouncementMessage(VehicleAnnouncementMessage),
    RoutingActivationRequest(RoutingActivationRequest),
    RoutingActivationResponse(RoutingActivationResponse),
    AliveCheckRequest(AliveCheckRequest),
    AliveCheckResponse(AliveCheckResponse),
    EntityStatusRequest(EntityStatusRequest),
    EntityStatusResponse(EntityStatusResponse),
    PowerInformationRequest(PowerInformationRequest),
    PowerInformationResponse(PowerInformationResponse),
    DiagnosticMessage(DiagnosticMessage<'a>),
    DiagnosticMessageAck(DiagnosticMessageAck<'a>),
    DiagnosticMessageNack(DiagnosticMessageNack),
}

impl<'a> Payload<'a> {
    pub fn decode(payload_type: PayloadType, buf: &mut &'a [u8]) -> Result<Self, DoipError> {
        match payload_type {
            PayloadType::GenericNack => Ok(Self::GenericNack(GenericNack::decode(buf)?)),
            PayloadType::VehicleIdentificationRequest => Ok(Self::VehicleIdentificationRequest(
                VehicleIdentificationRequest::decode(buf)?,
            )),
            PayloadType::VehicleIdentificationRequestEid => {
                Ok(Self::VehicleIdentificationRequestEid(
                    VehicleIdentificationRequestEid::decode(buf)?,
                ))
            }
            PayloadType::VehicleIdentificationRequestVin => {
                Ok(Self::VehicleIdentificationRequestVin(
                    VehicleIdentificationRequestVin::decode(buf)?,
                ))
            }
            PayloadType::VehicleAnnouncementMessage => Ok(Self::VehicleAnnouncementMessage(
                VehicleAnnouncementMessage::decode(buf)?,
            )),
            PayloadType::RoutingActivationRequest => Ok(Self::RoutingActivationRequest(
                RoutingActivationRequest::decode(buf)?,
            )),
            PayloadType::RoutingActivationResponse => Ok(Self::RoutingActivationResponse(
                RoutingActivationResponse::decode(buf)?,
            )),
            PayloadType::AliveCheckRequest => {
                Ok(Self::AliveCheckRequest(AliveCheckRequest::decode(buf)?))
            }
            PayloadType::AliveCheckResponse => {
                Ok(Self::AliveCheckResponse(AliveCheckResponse::decode(buf)?))
            }
            PayloadType::EntityStatusRequest => {
                Ok(Self::EntityStatusRequest(EntityStatusRequest::decode(buf)?))
            }
            PayloadType::EntityStatusResponse => Ok(Self::EntityStatusResponse(
                EntityStatusResponse::decode(buf)?,
            )),
            PayloadType::PowerInformationRequest => Ok(Self::PowerInformationRequest(
                PowerInformationRequest::decode(buf)?,
            )),
            PayloadType::PowerInformationResponse => Ok(Self::PowerInformationResponse(
                PowerInformationResponse::decode(buf)?,
            )),
            PayloadType::DiagnosticMessage => {
                Ok(Self::DiagnosticMessage(DiagnosticMessage::decode(buf)?))
            }
            PayloadType::DiagnosticMessageAck => Ok(Self::DiagnosticMessageAck(
                DiagnosticMessageAck::decode(buf)?,
            )),
            PayloadType::DiagnosticMessageNack => Ok(Self::DiagnosticMessageNack(
                DiagnosticMessageNack::decode(buf)?,
            )),
        }
    }
}

impl FrameWrite for Payload<'_> {
    type Error = DoipError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::GenericNack(inner) => inner.encode(buf),
            Self::VehicleIdentificationRequest(inner) => inner.encode(buf),
            Self::VehicleIdentificationRequestEid(inner) => inner.encode(buf),
            Self::VehicleIdentificationRequestVin(inner) => inner.encode(buf),
            Self::VehicleAnnouncementMessage(inner) => inner.encode(buf),
            Self::RoutingActivationRequest(inner) => inner.encode(buf),
            Self::RoutingActivationResponse(inner) => inner.encode(buf),
            Self::AliveCheckRequest(inner) => inner.encode(buf),
            Self::AliveCheckResponse(inner) => inner.encode(buf),
            Self::EntityStatusRequest(inner) => inner.encode(buf),
            Self::EntityStatusResponse(inner) => inner.encode(buf),
            Self::PowerInformationRequest(inner) => inner.encode(buf),
            Self::PowerInformationResponse(inner) => inner.encode(buf),
            Self::DiagnosticMessage(inner) => inner.encode(buf),
            Self::DiagnosticMessageAck(inner) => inner.encode(buf),
            Self::DiagnosticMessageNack(inner) => inner.encode(buf),
        }
    }
}
