//! Functional addressing and negative-response suppression.
//!
//! Enforces ISO 14229-1:2026 as recorded in ADR-005. The clause:
//!
//! > As required by the tables in the following subclauses, negative response messages
//! > with negative response codes of serviceNotSupported,
//! > serviceNotSupportedInActiveSession, and requestOutOfRange shall not be transmitted
//! > when functional addressing was used for the request message.
//!
//! and, stating the requirement in full while governing `0x78`:
//!
//! > When this NRC is used, the server shall always send a final response independent
//! > of the suppressPosRspMsgIndicationBit value or the suppress requirement for
//! > responses with NRCs SNS, SFNS, SNSIAS, SFNSIAS, and ROOR on functionally addressed
//! > requests.
//!
//! So the suppressed set is five: SNS `0x11`, SFNS `0x12`, ROOR `0x31`, SFNSIAS `0x7E`,
//! SNSIAS `0x7F`. The clause naming only three is an incomplete restatement.
//!
//! Each test names the server case it covers from the standard's functional-addressing
//! table. The quotes are here so the expected values have provenance rather than being
//! asserted on someone's recollection - which was wrong twice while settling this.

use ace_core::Vec;
use ace_core::clock::Duration;
use ace_core::clock::Instant;
use ace_core::io::{InboundFrame, NodeAddress};
use ace_server::BuiltinNrc;
use ace_server::config::{
    DidConfig, SecurityLevelConfig, ServerConfig, ServiceConfig, SessionConfig,
};
use ace_server::handler::ServerHandler;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_server::server::UdsServer;

// region: Fixture

const MAX_FRAME: usize = 32;
const MAX_OUTBOX: usize = 4;

const PHYSICAL: u16 = 0x1000;
const FUNCTIONAL: u16 = 0x7DF;
const TESTER: u32 = 0x0E80;

/// Returns `response_len` bytes for the one DID it supports.
struct Handler {
    response_len: usize,
}

impl ServerHandler for Handler {
    type Error = BuiltinNrc;

    fn read_did(&self, _did: u16, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(self.response_len.min(buf.len()))
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
    5_000,
    10_000,
    3,
    4,
    2,
    Handler,
    Provider,
>;

fn server(response_len: usize) -> TestServer {
    let config = ServerConfig::new(PHYSICAL, FUNCTIONAL)
        .with_session(SessionConfig::default_session())
        // 0x22 ReadDataByIdentifier and 0x10 DiagnosticSessionControl are supported;
        // everything else is not, which is what makes the "unknown SID" cases possible.
        .with_service(ServiceConfig::new(0x22, &[0x01]))
        .with_service(ServiceConfig::new(0x10, &[0x01]))
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
        Handler { response_len },
        Provider,
        NodeAddress(u32::from(PHYSICAL)),
    )
}

fn frame_to(dst: u16, data: &[u8]) -> InboundFrame<'_> {
    InboundFrame::new(NodeAddress(TESTER), NodeAddress(u32::from(dst)), data)
}

/// Runs one request and returns whatever the server put on the wire.
fn exchange(
    s: &mut TestServer,
    dst: u16,
    data: &[u8],
) -> Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX> {
    s.handle(&frame_to(dst, data), Instant::ZERO)
        .expect("handled");

    let mut out = Vec::new();
    s.drain_outbox(&mut out).expect("outbox fits");
    out
}

// endregion: Fixture

// region: Not addressed to this server

/// A frame addressed to neither this server's physical nor its functional address is
/// not ours, and must produce nothing at all.
///
/// Before node-side filtering existed this could not be expressed: the caller decided
/// what reached the node, so a real ECU had no filter of its own.
#[test]
fn frame_for_another_address_is_ignored_entirely() {
    let mut s = server(2);
    let out = exchange(&mut s, 0x1234, &[0x22, 0xF1, 0x90]);
    assert!(
        out.is_empty(),
        "a frame for 0x1234 is not this server's business"
    );
}

// endregion: Not addressed to this server

// region: Case a / f - supported and processed

/// Case (a): supported request, processed, SPRMIB = 0 - positive response.
#[test]
fn case_a_functional_supported_request_answers_positively() {
    let mut s = server(2);
    let out = exchange(&mut s, FUNCTIONAL, &[0x22, 0xF1, 0x90]);

    assert_eq!(out.len(), 1, "case (a) sends a positive response");
    assert_eq!(out[0].1[0], 0x62, "0x22 + 0x40");
}

// endregion: Case a / f

// region: Cases d, i - SID not supported

/// Cases (d) and (i): the SID is not supported, so the server does not respond -
/// SNS `0x11` is suppressed. On a broadcast most servers do not implement a given
/// service, so answering would flood the client.
#[test]
fn case_d_functional_unknown_service_is_silent() {
    let mut s = server(2);
    // 0x23 ReadMemoryByAddress is not configured.
    let out = exchange(&mut s, FUNCTIONAL, &[0x23, 0x00, 0x00]);
    assert!(
        out.is_empty(),
        "SNS must not be transmitted on a functional request"
    );
}

/// The same request addressed physically *must* be answered. This is the control: it
/// proves the silence above comes from the addressing rule and not from the server
/// simply failing to handle the request.
#[test]
fn physically_addressed_unknown_service_still_answers() {
    let mut s = server(2);
    let out = exchange(&mut s, PHYSICAL, &[0x23, 0x00, 0x00]);

    assert_eq!(out.len(), 1, "physical addressing is not suppressed");
    assert_eq!(out[0].1[0], 0x7F);
    assert_eq!(out[0].1[2], 0x11, "serviceNotSupported");
}

// endregion: Cases d, i

// region: Cases c, h - no supported data parameter

/// Cases (c) and (h): the service is supported but the requested data parameter is not,
/// which the server reports as ROOR `0x31` - suppressed on a functional request.
#[test]
fn case_c_functional_unknown_did_is_silent() {
    let mut s = server(2);
    let out = exchange(&mut s, FUNCTIONAL, &[0x22, 0xF1, 0x99]);
    assert!(
        out.is_empty(),
        "ROOR must not be transmitted on a functional request"
    );
}

#[test]
fn physically_addressed_unknown_did_still_answers() {
    let mut s = server(2);
    let out = exchange(&mut s, PHYSICAL, &[0x22, 0xF1, 0x99]);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].1[2], 0x31, "requestOutOfRange");
}

// endregion: Cases c, h

// region: Cases e, j - sub-function not supported

/// Cases (e) and (j): the SID is supported but the sub-function is not, so the server
/// stays silent.
///
/// This is the case the three-NRC sentence does not cover. SFNS `0x12` is absent from
/// it, yet the table requires silence here - which is why the five-NRC list from the
/// `0x78` clause is the authoritative one. Implementing from the shorter sentence would
/// wrongly answer a broadcast.
#[test]
fn case_e_functional_unsupported_subfunction_is_silent() {
    let mut s = server(2);
    // Session 0x04 is not configured; only the default session is.
    let out = exchange(&mut s, FUNCTIONAL, &[0x10, 0x04]);
    assert!(
        out.is_empty(),
        "SFNS must not be transmitted on a functional request"
    );
}

#[test]
fn physically_addressed_unsupported_subfunction_still_answers() {
    let mut s = server(2);
    let out = exchange(&mut s, PHYSICAL, &[0x10, 0x04]);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].1[2], 0x12, "subFunctionNotSupported");
}

// endregion: Cases e, j

// region: Cases b, g - implemented, but processing failed

/// Cases (b) and (g): the request *is* this server's and processing it failed, so the
/// negative response is transmitted even though the request was functionally addressed.
///
/// This is the distinction that makes a flat NRC blocklist the wrong model. The rule is
/// not "filter these codes", it is "distinguish not-mine from mine-and-broken".
///
/// The failure here is a handler returning more data than the frame can carry, which
/// yields NRC 0x14 responseTooLong - not one of the five, so it is never suppressed.
#[test]
fn case_b_functional_processing_failure_still_answers() {
    // 1 response SID + 2 DID bytes + 32 data bytes cannot fit MAX_FRAME = 32.
    let mut s = server(MAX_FRAME);
    let out = exchange(&mut s, FUNCTIONAL, &[0x22, 0xF1, 0x90]);

    assert_eq!(
        out.len(),
        1,
        "a processing failure is reported on a broadcast"
    );
    assert_eq!(
        out[0].1.as_slice(),
        &[0x7F, 0x22, 0x14],
        "responseTooLong is not in the suppressed set"
    );
}

// endregion: Cases b, g

// region: Suppression is per-request, not sticky

/// The addressing mode must not leak between requests.
///
/// A functional request that suppresses its NRC, followed by a physical request that
/// should produce one, has to answer the second. The mode is per-request state, and a
/// stale value would silently swallow responses the client is entitled to.
#[test]
fn suppression_does_not_leak_into_the_next_request() {
    let mut s = server(2);

    let quiet = exchange(&mut s, FUNCTIONAL, &[0x23, 0x00, 0x00]);
    assert!(quiet.is_empty(), "functional request suppressed");

    let loud = exchange(&mut s, PHYSICAL, &[0x23, 0x00, 0x00]);
    assert_eq!(loud.len(), 1, "the following physical request must answer");
    assert_eq!(loud[0].1[2], 0x11);
}

// endregion: Suppression is per-request, not sticky
