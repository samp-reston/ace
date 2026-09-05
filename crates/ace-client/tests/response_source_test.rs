//! A client accepts a response only from the server it addressed, and only if the response
//! is addressed to it.
//!
//! Both checks were absent, and neither is observable through the deterministic scenario: it
//! runs one tester, so a frame for the wrong client never exists. These assert the rule
//! directly, where the addresses can be chosen.
//!
//! The failure mode being prevented is worse than a missing response. A client that accepts
//! any frame completes its pending request with a payload meant for somebody else, and
//! reports it as that request's answer - so the caller receives another tester's data instead
//! of the timeout it was owed.

use ace_client::client::UdsClient;
use ace_client::config::ClientConfig;
use ace_client::event::ClientEvent;
use ace_core::clock::{Duration, Instant};
use ace_core::io::{InboundFrame, NodeAddress};

const MAX_FRAME: usize = 64;
const MAX_OUTBOX: usize = 4;
const PENDING: usize = 4;
const MAX_EVENTS: usize = 8;
const PERIODIC: usize = 4;
const MAX_DATA: usize = 64;

const CLIENT: u16 = 0x0E80;
const SERVER: u16 = 0x1726;

/// Another server on the same segment, which this client never addressed.
const STRANGER: u16 = 0x14F0;

/// Another tester on the same segment.
const OTHER_CLIENT: u16 = 0x0E81;

const P2: u64 = 150;

type TestClient = UdsClient<MAX_FRAME, MAX_OUTBOX, PENDING, MAX_EVENTS, PERIODIC, MAX_DATA>;

/// A client with one outstanding ReadDataByIdentifier.
fn client_awaiting_response() -> TestClient {
    let config = ClientConfig::new(CLIENT, SERVER).with_p2_timeout(Duration::from_millis(P2));
    let mut client = TestClient::new(config, NodeAddress(u32::from(CLIENT)));

    client
        .request(&[0x22, 0xF1, 0x90], Instant::ZERO)
        .expect("request accepted");

    let mut outbox = ace_core::Vec::new();
    client.drain_outbox(&mut outbox).expect("outbox drains");

    assert_eq!(client.pending_count(), 1, "the request is outstanding");
    client
}

/// A positive ReadDataByIdentifier response.
const RESPONSE: [u8; 5] = [0x62, 0xF1, 0x90, 0xAA, 0xBB];

fn frame(src: u16, dst: u16) -> InboundFrame<'static> {
    InboundFrame::new(
        NodeAddress(u32::from(src)),
        NodeAddress(u32::from(dst)),
        &RESPONSE,
    )
}

/// The baseline: a response from the addressed server, to this client, completes the request.
///
/// Without this the tests below would pass on a client that rejected everything.
#[test]
fn a_response_from_the_addressed_server_is_accepted() {
    let mut client = client_awaiting_response();

    client
        .handle(&frame(SERVER, CLIENT), Instant::ZERO)
        .expect("handled");

    assert_eq!(client.pending_count(), 0, "the request completed");

    let events: std::vec::Vec<_> = client.drain_events().collect();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ClientEvent::PositiveResponse { sid: 0x22, .. })),
        "expected a positive response event, got {events:?}"
    );
}

/// A response from a server this client never addressed must not complete its request.
#[test]
fn a_response_from_another_server_is_ignored() {
    let mut client = client_awaiting_response();

    client
        .handle(&frame(STRANGER, CLIENT), Instant::ZERO)
        .expect("handled");

    assert_eq!(
        client.pending_count(),
        1,
        "a stranger's response must not complete this client's request"
    );
    assert!(
        !client.has_events(),
        "and must not be reported as this request's answer"
    );
}

/// A response addressed to a different client must not be consumed.
#[test]
fn a_response_for_another_client_is_ignored() {
    let mut client = client_awaiting_response();

    client
        .handle(&frame(SERVER, OTHER_CLIENT), Instant::ZERO)
        .expect("handled");

    assert_eq!(
        client.pending_count(),
        1,
        "a response addressed to another tester is not this client's"
    );
    assert!(!client.has_events());
}

/// The consequence of ignoring it: the request still times out.
///
/// This is the point of the whole check. Silently accepting a foreign response would replace
/// this timeout with a wrong answer, and the caller would have no way to tell.
#[test]
fn an_ignored_response_still_lets_the_request_time_out() {
    let mut client = client_awaiting_response();

    client
        .handle(&frame(STRANGER, CLIENT), Instant::ZERO)
        .expect("handled");

    let after_p2 = Instant::ZERO + Duration::from_millis(P2 + 1);
    client.tick(after_p2).expect("tick");

    let events: std::vec::Vec<_> = client.drain_events().collect();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ClientEvent::Timeout { sid: 0x22 })),
        "expected the request to time out, got {events:?}"
    );
}
