// crates/ace-doip/tests/roundtrip.rs
//
// Round-trip property tests: for any byte sequence that decodes successfully,
// encode → decode must produce the original value.
//
// Tests are organised into two layers:
//   1. Inner structs - each concrete payload type tested directly.
//   2. Entry point - DoipMessage tested via the top-level decoder.
//
// Proptest strategy: throw arbitrary byte slices (0..=256 bytes) at decode.
// Invalid frames are silently skipped - we only assert on values that decoded
// successfully. This catches encoder/decoder asymmetry without needing to
// generate structurally valid DoIP frames.

use ace_core::codec::{FrameRead, FrameWrite};
use ace_doip::message::*;
use ace_doip::payload::*;
use bytes::BytesMut;
use proptest::prelude::*;

// region: Helpers

fn encode<T: FrameWrite>(value: &T) -> BytesMut
where
    T::Error: core::fmt::Debug,
{
    let mut buf = BytesMut::new();
    value
        .encode(&mut buf)
        .expect("encode should not fail on a successfully decoded value");
    buf
}

fn try_decode<'a, T: FrameRead<'a>>(bytes: &'a [u8]) -> Option<T> {
    let mut cursor = bytes;
    T::decode(&mut cursor).ok()
}

// endregion: Helpers

// region: Round-trip macro

macro_rules! roundtrip {
    ($name:ident, $ty:ty) => {
        proptest! {
            #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]
            #[test]
            fn $name(bytes in proptest::collection::vec(any::<u8>(), 0..=256usize)) {
                if let Some(first) = try_decode::<$ty>(&bytes) {
                    let encoded = encode(&first);
                    let second = try_decode::<$ty>(&encoded)
                        .expect("re-decode of encoded value must succeed");
                    prop_assert_eq!(first, second);
                }
            }
        }
    };
}

// endregion: Round-trip macro

// region: Inner payload types

roundtrip!(rt_generic_nack, GenericNack);
roundtrip!(
    rt_vehicle_identification_request,
    VehicleIdentificationRequest
);
roundtrip!(
    rt_vehicle_identification_request_eid,
    VehicleIdentificationRequestEid
);
roundtrip!(
    rt_vehicle_identification_request_vin,
    VehicleIdentificationRequestVin
);
roundtrip!(rt_vehicle_announcement_message, VehicleAnnouncementMessage);
roundtrip!(rt_routing_activation_request, RoutingActivationRequest);
roundtrip!(rt_routing_activation_response, RoutingActivationResponse);
roundtrip!(rt_alive_check_request, AliveCheckRequest);
roundtrip!(rt_alive_check_response, AliveCheckResponse);
roundtrip!(rt_entity_status_request, EntityStatusRequest);
roundtrip!(rt_entity_status_response, EntityStatusResponse);
roundtrip!(rt_power_information_request, PowerInformationRequest);
roundtrip!(rt_power_information_response, PowerInformationResponse);
roundtrip!(rt_diagnostic_message, DiagnosticMessage);
roundtrip!(rt_diagnostic_message_ack, DiagnosticMessageAck);
roundtrip!(rt_diagnostic_message_nack, DiagnosticMessageNack);

// endregion: Inner payload types

// region: Entry point - DoipMessage
//
// Tests the full decode path including header parsing, payload type dispatch,
// and payload construction. Uses raw byte slices since DoipMessage::decode
// reads the full 8-byte header itself.

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]
    #[test]
    fn rt_doip_message(bytes in proptest::collection::vec(any::<u8>(), 0..=256usize)) {
        if let Some(first) = try_decode::<DoipMessage>(&bytes) {
            let encoded = encode(&first);
            let second = try_decode::<DoipMessage>(&encoded)
                .expect("re-decode of encoded DoipMessage must succeed");
            prop_assert_eq!(first, second);
        }
    }
}

// endregion: Entry point - DoipMessage
