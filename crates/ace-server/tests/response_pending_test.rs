//! `0x78` requestCorrectlyReceivedResponsePending.
//!
//! Until now this NRC was declared, implemented for `BuiltinNrc`, and never once emitted: the
//! server advertised a P2* budget in its DiagnosticSessionControl response and then had no
//! pending state, no P2* deadline, and no way for a handler to say "accepted, not finished".
//! The client already handled receiving it.
//!
//! The governing clause, from ISO 14229-1:2026 and recorded in ADR-005:
//!
//! > When this NRC is used, the server shall always send a final response independent of the
//! > suppressPosRspMsgIndicationBit value or the suppress requirement for responses with NRCs
//! > SNS, SFNS, SNSIAS, SFNSIAS, and ROOR on functionally addressed requests.
//!
//! That makes `0x78` an override of both suppression rules, and makes the obligation *state*
//! rather than a property of any frame - which is why a stateless predicate could not express
//! it and why the tests below check what happens after the request is gone.

use ace_core::Vec;
use ace_core::clock::{Duration, Instant};
use ace_core::io::{InboundFrame, NodeAddress};
use ace_server::BuiltinNrc;
use ace_server::config::{
    DidConfig, SecurityLevelConfig, ServerConfig, ServiceConfig, SessionConfig,
};
use ace_server::handler::ServerHandler;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_server::server::{ServerError, UdsServer};

// region: Fixture

const MAX_FRAME: usize = 32;
const MAX_OUTBOX: usize = 4;

const PHYSICAL: u16 = 0x1000;
const FUNCTIONAL: u16 = 0x7DF;
const TESTER: u32 = 0x0E80;

/// P2* for the default session, in milliseconds.
const P2_STAR_MS: u64 = 2_000;

/// Reports work as still in progress until told otherwise.
struct Handler {
    still_working: bool,
}

impl ServerHandler for Handler {
    type Error = BuiltinNrc;

    fn read_did(&self, _did: u16, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.still_working {
            return Err(BuiltinNrc::ResponsePending);
        }

        if buf.len() < 2 {
            return Err(BuiltinNrc::ResponseTooLong);
        }

        buf[0] = 0xAA;
        buf[1] = 0xBB;
        Ok(2)
    }

    fn write_did(&mut self, _did: u16, _data: &[u8]) -> Result<(), Self::Error> {
        Err(BuiltinNrc::RequestOutOfRange)
    }

    fn ecu_reset(&mut self, _reset_type: u8) -> Result<(), Self::Error> {
        Err(BuiltinNrc::ServiceNotSupported)
    }
}

struct Provider;

impl SecurityProvider for Provider {
    fn generate_seed(&mut self, _level: u8, buf: &mut [u8]) -> Result<usize, SecurityError> {
        if buf.is_empty() {
            return Err(SecurityError::InvalidKey);
        }
        buf[0] = 0xAA;
        Ok(1)
    }

    fn validate_key(&self, _level: u8, _seed: &[u8], _key: &[u8]) -> Result<(), SecurityError> {
        Err(SecurityError::InvalidKey)
    }
}

type TestServer = UdsServer<
    MAX_FRAME,
    MAX_OUTBOX,
    2,
    4,
    2,
    4,
    5_000,
    50,
    P2_STAR_MS,
    10_000,
    3,
    4,
    2,
    Handler,
    Provider,
>;

fn server() -> TestServer {
    // No builder for P2* - the field is public and the default sessions all use 5 s.
    let mut default_session = SessionConfig::default_session();
    default_session.p2_extended_timeout = Duration::from_millis(P2_STAR_MS);

    let config = ServerConfig::new(PHYSICAL, FUNCTIONAL)
        .with_session(default_session)
        .with_service(ServiceConfig::new(0x22, &[0x01]))
        .with_did(DidConfig::read_only(0xF190, &[0x01]))
        .with_security_level(SecurityLevelConfig {
            level: 0x01,
            max_attempts: 3,
            lockout_duration: Duration::from_millis(10_000),
            seed_length: 1,
            key_length: 1,
        });

    UdsServer::new(
        config,
        Handler {
            still_working: true,
        },
        Provider,
        NodeAddress(u32::from(PHYSICAL)),
    )
}

fn out(s: &mut TestServer) -> Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX> {
    let mut buf = Vec::new();
    s.drain_outbox(&mut buf).expect("outbox fits");
    buf
}

/// Sends a ReadDataByIdentifier for the one supported DID.
///
/// ReadDataByIdentifier carries no sub-function byte, so there is no suppress-positive bit to
/// set on it. The clause's suppress-bit clause is covered by the functional-addressing test.
fn read_did(s: &mut TestServer, dst: u16) {
    let request = [0x22, 0xF1, 0x90];
    let frame = InboundFrame::new(NodeAddress(TESTER), NodeAddress(u32::from(dst)), &request);
    s.handle(&frame, Instant::ZERO).expect("handled");
}

// endregion: Fixture

// region: The promise is made

/// A handler reporting work in progress produces `0x78` and leaves the request outstanding.
#[test]
fn a_handler_reporting_pending_work_emits_0x78() {
    let mut s = server();
    read_did(&mut s, PHYSICAL);

    let sent = out(&mut s);
    assert_eq!(sent.len(), 1, "the client is told the request was accepted");
    assert_eq!(
        sent[0].1.as_slice(),
        &[0x7F, 0x22, 0x78],
        "requestCorrectlyReceivedResponsePending for the requested SID"
    );
    assert!(
        s.has_pending_request(),
        "the obligation to answer outlives the request"
    );
}

// endregion: The promise is made

// region: The promise is kept

/// The final response carries the original SID and reaches the original requester, even
/// though no frame is in hand when the work finishes.
#[test]
fn completing_a_pending_request_answers_the_original_requester() {
    let mut s = server();
    read_did(&mut s, PHYSICAL);
    let _ = out(&mut s);

    s.complete_pending(&[0xF1, 0x90, 0xAA], Instant::ZERO)
        .expect("a request was pending");

    let sent = out(&mut s);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0.0, TESTER, "answered to whoever asked");
    assert_eq!(sent[0].1[0], 0x62, "0x22 + 0x40");
    assert!(!s.has_pending_request(), "the obligation is discharged");
}

/// Work that was accepted and then failed still gets an answer.
#[test]
fn failing_a_pending_request_still_answers() {
    let mut s = server();
    read_did(&mut s, PHYSICAL);
    let _ = out(&mut s);

    s.fail_pending(BuiltinNrc::ConditionsNotCorrect, Instant::ZERO)
        .expect("a request was pending");

    let sent = out(&mut s);
    assert_eq!(sent[0].1.as_slice(), &[0x7F, 0x22, 0x22]);
    assert!(!s.has_pending_request());
}

/// Answering when nothing is outstanding is reported, not ignored.
#[test]
fn completing_with_nothing_pending_is_an_error() {
    let mut s = server();

    let result = s.complete_pending(&[0x00], Instant::ZERO);
    assert!(
        matches!(result, Err(ServerError::NoPendingRequest)),
        "a caller that has lost track of its own work should hear about it"
    );
}

// endregion: The promise is kept

// region: The promise is renewed

/// P2* expiring re-sends `0x78` rather than letting the client time out on a request the
/// server still intends to answer.
#[test]
fn p2_star_expiry_re_sends_0x78() {
    let mut s = server();
    read_did(&mut s, PHYSICAL);
    let _ = out(&mut s);

    // Just before the deadline, nothing more is due.
    s.tick(Instant::ZERO + Duration::from_millis(P2_STAR_MS - 1))
        .expect("tick");
    assert!(out(&mut s).is_empty(), "P2* has not expired yet");

    s.tick(Instant::ZERO + Duration::from_millis(P2_STAR_MS))
        .expect("tick");

    let sent = out(&mut s);
    assert_eq!(sent.len(), 1, "the promise is renewed");
    assert_eq!(sent[0].1.as_slice(), &[0x7F, 0x22, 0x78]);
    assert!(s.has_pending_request(), "still outstanding");
}

// endregion: The promise is renewed

// region: One request at a time

/// A second request while one is pending is refused with busyRepeatRequest.
///
/// Interleaving would leave two requests competing for one pending slot, and the first -
/// already promised an answer - would never receive one.
#[test]
fn a_second_request_while_pending_is_refused() {
    let mut s = server();
    read_did(&mut s, PHYSICAL);
    let _ = out(&mut s);

    read_did(&mut s, PHYSICAL);

    let sent = out(&mut s);
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].1[2], 0x21,
        "busyRepeatRequest - retry, the request itself was fine"
    );
    assert!(
        s.has_pending_request(),
        "the first request is still the one owed an answer"
    );
}

// endregion: One request at a time

// region: Precedence over both suppression rules

/// `0x78` and its final response are sent on a functionally addressed request.
///
/// The five "not implemented" codes are withheld on a broadcast, but `0x78` is not one of
/// them and the clause makes the final response mandatory regardless. A promise that could
/// itself be suppressed would commit the server to an answer the client never learns to
/// expect.
#[test]
fn a_functionally_addressed_request_still_gets_0x78_and_its_final_response() {
    let mut s = server();
    read_did(&mut s, FUNCTIONAL);

    let sent = out(&mut s);
    assert_eq!(
        sent[0].1.as_slice(),
        &[0x7F, 0x22, 0x78],
        "0x78 is not suppressed on a functional request"
    );

    s.complete_pending(&[0xF1, 0x90, 0xAA], Instant::ZERO)
        .expect("a request was pending");

    let sent = out(&mut s);
    assert_eq!(
        sent.len(),
        1,
        "the final response is mandatory once 0x78 has been sent"
    );
    assert_eq!(sent[0].1[0], 0x62);
}

// endregion: Precedence over both suppression rules
