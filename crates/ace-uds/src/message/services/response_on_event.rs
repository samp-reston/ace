use ace_core::{DiagError, FrameIter, FrameRead, FrameWrite};
use ace_macros::FrameCodec;

use crate::{UdsError, ValidationError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResponseOnEventRequest<'a> {
    pub storage_state: StorageState,
    pub event_type: EventType<'a>,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum StorageState {
    #[frame(id = 0x00)]
    DoNotStoreEvent,
    #[frame(id = 0x01)]
    StoreEvent,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum EventType<'a> {
    #[frame(id = 0x00)]
    StopResponseOnEvent(StopResponseOnEvent),
    #[frame(id = 0x01)]
    OnDtcStatusChange(OnDtcStatusChange<'a>),
    #[frame(id = 0x03)]
    OnChangeOfDataIdentifier(OnChangeOfDataIdentifier<'a>),
    #[frame(id = 0x04)]
    ReportActivatedEvents(ReportActivatedEvents),
    #[frame(id = 0x05)]
    StartResponseOnEvent(StartResponseOnEvent),
    #[frame(id = 0x06)]
    ClearResponseOnEvent(ClearResponseOnEvent),
    #[frame(id = 0x07)]
    OnComparisonOfValues(OnComparisonOfValues<'a>),
    #[frame(id = 0x08)]
    ReportMostRecentDtcOnStatusChange(ReportMostRecentDtcOnStatusChange),
    #[frame(id = 0x09)]
    ReportDtcRecordInformationOnDtcStatusChange(ReportDtcRecordInformationOnDtcStatusChange<'a>),
    #[frame(id_pat = "0x02 | 0x0A..=0x3F")]
    IsoSaeReserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct StopResponseOnEvent {
    pub event_window_time: EventWindowTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct OnDtcStatusChange<'a> {
    pub event_window_time: EventWindowTime,
    pub dtc_status_mask: u8,
    pub service_to_respond_to_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct OnChangeOfDataIdentifier<'a> {
    pub event_window_time: EventWindowTime,
    pub data_identifier: u16,
    pub service_to_respond_to_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportActivatedEvents {
    pub event_window_time: EventWindowTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct StartResponseOnEvent {
    pub event_window_time: EventWindowTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ClearResponseOnEvent {
    pub event_window_time: EventWindowTime,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct OnComparisonOfValues<'a> {
    pub event_window_time: EventWindowTime,
    pub event_type_record: [u8; 10],
    pub service_to_response_to_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportMostRecentDtcOnStatusChange {
    pub event_window_time: EventWindowTime,
    pub dtc_status_mask: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportDtcRecordInformationOnDtcStatusChange<'a> {
    pub event_window_time: EventWindowTime,
    pub dtc_status_mask: u8,
    pub read_dtc_information_sub_function: u8,
    pub read_dtc_information_parameters: &'a [u8],
}

impl<'a> ace_core::codec::FrameRead<'a> for ResponseOnEventRequest<'a> {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        let storage_state = match (byte & 0x40) >> 6 {
            0x00 => StorageState::DoNotStoreEvent,
            _ => StorageState::StoreEvent,
        };

        let event_type = match byte & 0x3F {
            0x00 => EventType::StopResponseOnEvent(StopResponseOnEvent::decode(buf)?),
            0x01 => EventType::OnDtcStatusChange(OnDtcStatusChange::decode(buf)?),
            0x03 => EventType::OnChangeOfDataIdentifier(OnChangeOfDataIdentifier::decode(buf)?),
            0x04 => EventType::ReportActivatedEvents(ReportActivatedEvents::decode(buf)?),
            0x05 => EventType::StartResponseOnEvent(StartResponseOnEvent::decode(buf)?),
            0x06 => EventType::ClearResponseOnEvent(ClearResponseOnEvent::decode(buf)?),
            0x07 => EventType::OnComparisonOfValues(OnComparisonOfValues::decode(buf)?),
            0x08 => EventType::ReportMostRecentDtcOnStatusChange(
                ReportMostRecentDtcOnStatusChange::decode(buf)?,
            ),
            0x09 => EventType::ReportDtcRecordInformationOnDtcStatusChange(
                ReportDtcRecordInformationOnDtcStatusChange::decode(buf)?,
            ),
            v @ (0x02 | 0x0A..=0x3F) => EventType::IsoSaeReserved(v),
            val => return Err(UdsError::Validation(ValidationError::InvalidEventType(val))),
        };

        Ok(Self {
            storage_state,
            event_type,
        })
    }
}

impl ace_core::codec::FrameWrite for ResponseOnEventRequest<'_> {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let storage_bit: u8 = match self.storage_state {
            StorageState::DoNotStoreEvent => 0x00,
            StorageState::StoreEvent => 0x40,
        };

        let event_bits: u8 = match &self.event_type {
            EventType::StopResponseOnEvent(_) => 0x00,
            EventType::OnDtcStatusChange(_) => 0x01,
            EventType::OnChangeOfDataIdentifier(_) => 0x03,
            EventType::ReportActivatedEvents(_) => 0x04,
            EventType::StartResponseOnEvent(_) => 0x05,
            EventType::ClearResponseOnEvent(_) => 0x06,
            EventType::OnComparisonOfValues(_) => 0x07,
            EventType::ReportMostRecentDtcOnStatusChange(_) => 0x08,
            EventType::ReportDtcRecordInformationOnDtcStatusChange(_) => 0x09,
            EventType::IsoSaeReserved(v) => *v,
        };

        buf.write_bytes(&[storage_bit | event_bits])
            .map_err(|e| UdsError::from(e))?;

        match &self.event_type {
            EventType::StopResponseOnEvent(inner) => inner.encode(buf)?,
            EventType::OnDtcStatusChange(inner) => inner.encode(buf)?,
            EventType::OnChangeOfDataIdentifier(inner) => inner.encode(buf)?,
            EventType::ReportActivatedEvents(inner) => inner.encode(buf)?,
            EventType::StartResponseOnEvent(inner) => inner.encode(buf)?,
            EventType::ClearResponseOnEvent(inner) => inner.encode(buf)?,
            EventType::OnComparisonOfValues(inner) => inner.encode(buf)?,
            EventType::ReportMostRecentDtcOnStatusChange(inner) => inner.encode(buf)?,
            EventType::ReportDtcRecordInformationOnDtcStatusChange(inner) => inner.encode(buf)?,
            EventType::IsoSaeReserved(_) => {}
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResponseOnEventResponse<'a> {
    ReportActivatedEventsResponse(ReportActivatedEventsResponse<'a>),
    AllButReportActivatedEvents(AllButReportActivatedEvents<'a>),
}

impl<'a> FrameRead<'a> for ResponseOnEventResponse<'a> {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        // Peek at the event type byte - do not consume it.
        // Both inner structs read it themselves as their first field.
        let event_type = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;

        if event_type == 0x04 {
            Ok(Self::ReportActivatedEventsResponse(
                ReportActivatedEventsResponse::decode(buf)?,
            ))
        } else {
            Ok(Self::AllButReportActivatedEvents(
                AllButReportActivatedEvents::decode(buf)?,
            ))
        }
    }
}

impl FrameWrite for ResponseOnEventResponse<'_> {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::ReportActivatedEventsResponse(inner) => inner.encode(buf),
            Self::AllButReportActivatedEvents(inner) => inner.encode(buf),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum EventTypeValue {
    #[frame(id = 0x00)]
    StopResponseOnEvent,
    #[frame(id = 0x01)]
    OnDtcStatusChange,
    #[frame(id = 0x03)]
    OnChangeOfDataIdentifier,
    #[frame(id = 0x04)]
    ReportActivatedEvents,
    #[frame(id = 0x05)]
    StartResponseOnEvent,
    #[frame(id = 0x06)]
    ClearResponseOnEvent,
    #[frame(id = 0x07)]
    OnComparisonOfValues,
    #[frame(id = 0x08)]
    ReportMostRecentDtcOnStatusChange,
    #[frame(id = 0x09)]
    ReportDtcRecordInformationOnDtcStatusChange,
    #[frame(id_pat = "0x02 | 0x0A..=0x3F")]
    IsoSaeReserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct AllButReportActivatedEvents<'a> {
    pub event_type: EventTypeValue,
    pub number_of_identified_events: u8,
    pub event_window_time: EventWindowTime,
    pub remaining: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReportActivatedEventsResponse<'a> {
    pub event_type: EventTypeValue,
    pub number_of_activated_events: u8,
    pub events: FrameIter<'a, Event<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct Event<'a> {
    pub event_window_time: EventWindowTime,
    pub remaining: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum EventWindowTime {
    #[frame(id_pat = "0x00..=0x01 | 0x09..=0xFF")]
    IsoSaeReserved(u8),
    #[frame(id = 0x02)]
    InfiniteTimeToResponse,
    #[frame(id = 0x03)]
    ShortEventWindowTime,
    #[frame(id = 0x04)]
    MediumEventWindowTime,
    #[frame(id = 0x05)]
    LongEventWindowTime,
    #[frame(id = 0x06)]
    PowerWindowTime,
    #[frame(id = 0x07)]
    IgnitionWindowTime,
    #[frame(id = 0x08)]
    ManufacturerTriggerEventWindowTime,
}
