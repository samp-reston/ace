//! Capacity-exhaustion tests.
//!
//! Phase 3's exit criterion is that a bounded input always yields an explicit
//! terminal result, never a truncated one. Nothing verified that until these tests:
//! every capacity path in the suite was unreachable by the existing suite, because
//! the DST harness runs with an allocator and, before the storage newtype landed,
//! the alloc-backed `Vec` grew without bound and never reported a full buffer.
//!
//! These deliberately run in `ace-server`'s **default** feature set, which is
//! `default = []` - no allocator. That is the configuration where the bounds are
//! real, so it is the only one where these assertions mean anything.

use ace_core::Vec;
use ace_core::clock::{Duration, Instant};
use ace_core::io::{InboundFrame, NodeAddress};
use ace_server::BuiltinNrc;
use ace_server::config::{
    DidConfig, SecurityLevelConfig, ServerConfig, ServiceConfig, SessionConfig,
};
use ace_server::handler::ServerHandler;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_server::server::UdsServer;

// region: Fixtures

/// Frame budget deliberately small so a handler can outgrow it.
const MAX_FRAME: usize = 8;
const MAX_OUTBOX: usize = 4;

/// Returns `RESPONSE_LEN` bytes for any readable DID, regardless of frame budget.
///
/// A real handler behaving this way is the case that used to corrupt the wire: the
/// server truncated the response and reported success.
struct FatHandler {
    response_len: usize,
}

impl ServerHandler for FatHandler {
    type Error = BuiltinNrc;

    fn read_did(&self, _did: u16, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = self.response_len.min(buf.len());
        for (i, b) in buf.iter_mut().enumerate().take(len) {
            *b = i as u8;
        }
        Ok(len)
    }

    fn write_did(&mut self, _did: u16, _data: &[u8]) -> Result<(), Self::Error> {
        Err(BuiltinNrc::RequestOutOfRange)
    }

    fn ecu_reset(&mut self, _reset_type: u8) -> Result<(), Self::Error> {
        Err(BuiltinNrc::ServiceNotSupported)
    }
}

/// Always rejects the key, so every SendKey is a failed attempt.
struct RejectingProvider;

impl SecurityProvider for RejectingProvider {
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
    2,  // MAX_SESSIONS
    4,  // MAX_SERVICES
    2,  // MAX_DIDS
    16, // MAX_SECURITY_LEVELS - above the 8-slot bookkeeping bound on purpose
    5_000,
    50,
    5_000,
    10_000,
    3, // DEFAULT_MAX_SECURITY_ATTEMPTS
    4, // MAX_SEED
    2, // MAX_PERIODIC
    FatHandler,
    RejectingProvider,
>;

fn server(response_len: usize) -> TestServer {
    let address = NodeAddress(0x1000);

    let mut config = ServerConfig::new(0x1000, 0x7DF)
        .with_session(SessionConfig::default_session())
        .with_service(ServiceConfig::new(0x22, &[0x01]))
        .with_service(ServiceConfig::new(0x27, &[0x01]))
        .with_did(DidConfig::read_only(0xF190, &[0x01]));

    // More levels than the 8 the server can track, so the last ones exercise the
    // bookkeeping bound.
    for i in 0..12u8 {
        config = config.with_security_level(SecurityLevelConfig {
            level: i * 2 + 1,
            max_attempts: 3,
            lockout_duration: Duration::from_millis(10_000),
            seed_length: 1,
            key_length: 1,
        });
    }

    UdsServer::new(
        config,
        FatHandler { response_len },
        RejectingProvider,
        address,
    )
}

/// A physically addressed request from the tester to this server.
///
/// These tests are about capacity, not addressing, so every request is physical - the
/// functional path and its suppression rules are covered in functional_addressing_test.
fn physical(data: &[u8]) -> InboundFrame<'_> {
    InboundFrame::new(NodeAddress(0x0E80), NodeAddress(0x1000), data)
}

fn drain(s: &mut TestServer) -> Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX> {
    let mut out = Vec::new();
    s.drain_outbox(&mut out).expect("outbox fits");
    out
}

// endregion: Fixtures

// region: Response too long

/// A handler returning more than the frame can carry must produce NRC 0x14, not a
/// positive response with its data field silently missing.
#[test]
fn oversized_handler_response_yields_nrc_response_too_long() {
    // 2 DID echo bytes + 8 data bytes cannot fit MAX_FRAME = 8.
    let mut s = server(MAX_FRAME);

    s.handle(&physical(&[0x22, 0xF1, 0x90]), Instant::ZERO)
        .expect("request is handled, the response is negative");

    let out = drain(&mut s);
    assert_eq!(out.len(), 1, "exactly one response frame");

    let frame = &out[0].1;
    assert_eq!(
        frame.as_slice(),
        &[0x7F, 0x22, 0x14],
        "expected negative response 0x7F 0x22 0x14 (responseTooLong), got {:?}",
        frame.as_slice()
    );
}

/// The same request with a response that does fit must still succeed, so the test
/// above is proving a boundary and not merely a broken server.
#[test]
fn response_within_budget_still_yields_positive_response() {
    // 1 response SID + 2 DID bytes + 4 data bytes = 7 <= MAX_FRAME.
    let mut s = server(4);

    s.handle(&physical(&[0x22, 0xF1, 0x90]), Instant::ZERO)
        .expect("request handled");

    let out = drain(&mut s);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].1[0], 0x62, "positive response to 0x22");
    assert_eq!(out[0].1.len(), 7, "SID + DID echo + 4 data bytes");
}

// endregion: Response too long

// region: Drain contract

/// Draining into a buffer that cannot hold everything must fail without moving
/// anything, so the caller can retry instead of losing frames to a count that
/// overstates what was delivered.
///
/// Note the shape of the real hazard. `drain_outbox` takes `&mut Vec<_, MAX_OUTBOX>`,
/// the *same* bound as the server's own outbox, so the type system already rules out
/// passing a smaller buffer. What it cannot rule out is a caller reusing a buffer
/// that still holds earlier frames, which is what this exercises.
#[test]
fn drain_into_partly_full_buffer_moves_nothing_and_reports_error() {
    let mut s = server(2);

    // Two queued responses.
    for _ in 0..2 {
        s.handle(&physical(&[0x22, 0xF1, 0x90]), Instant::ZERO)
            .expect("request handled");
    }

    // A buffer the caller forgot to clear: MAX_OUTBOX - 1 slots already taken.
    let mut reused: Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX> = Vec::new();
    for _ in 0..(MAX_OUTBOX - 1) {
        reused
            .push((NodeAddress(0xDEAD), Vec::new()))
            .expect("priming the buffer");
    }

    assert!(
        s.drain_outbox(&mut reused).is_err(),
        "one free slot cannot take two frames"
    );
    assert_eq!(
        reused.len(),
        MAX_OUTBOX - 1,
        "all-or-nothing: a refused drain must not partially fill the buffer"
    );

    // The frames are still in the server, so nothing was lost by the refusal.
    let recovered = drain(&mut s);
    assert_eq!(recovered.len(), 2, "both frames survived the refused drain");
}

// endregion: Drain contract

// region: Security bookkeeping fails closed

/// Every configured security level must enforce its attempt limit, including levels
/// beyond the 8 the server used to be able to track.
///
/// `SecurityState` previously fixed both bookkeeping tables at 8 entries while the
/// consumer separately declared `MAX_SECURITY_LEVELS`. A 12-level ECU therefore had
/// four levels whose failures were never recorded, and since `failed_count` returns 0
/// for an untracked level, those four accepted unlimited key guesses. This exercises
/// the 12th level specifically: it is the one that used to fail open.
#[test]
fn attempt_limit_is_enforced_on_levels_beyond_the_old_eight_slot_bound() {
    let mut s = server(1);

    // Occupy the first eight levels, exactly filling the table as it used to be
    // sized. Without this the test would pass against the old code too, since a
    // single level always fitted.
    for i in 0..8u8 {
        let level = i * 2 + 1;
        s.handle(&physical(&[0x27, level]), Instant::ZERO)
            .expect("seed request handled");
        let _ = drain(&mut s);
        s.handle(&physical(&[0x27, level + 1, 0x00]), Instant::ZERO)
            .expect("key request handled");
        let _ = drain(&mut s);
    }

    // The 9th distinct level is the first that the old bound could not record.
    let level = 8 * 2 + 1;

    // max_attempts is 3, so the first two rejections are invalidKey and the third
    // must escalate to exceededNumberOfAttempts. Under the old bound this level was
    // never counted, so it would report invalidKey indefinitely.
    for (attempt, want) in [0x35u8, 0x35, 0x36].iter().enumerate() {
        s.handle(&physical(&[0x27, level]), Instant::ZERO)
            .expect("seed request handled");
        let _ = drain(&mut s);

        s.handle(&physical(&[0x27, level + 1, 0x00]), Instant::ZERO)
            .expect("key request handled");
        let out = drain(&mut s);

        assert_eq!(out.len(), 1, "one response per SendKey");
        assert_eq!(
            out[0].1.as_slice(),
            &[0x7F, 0x27, *want],
            "attempt {} on level 0x{:02X}: expected NRC 0x{:02X}, got {:?}",
            attempt + 1,
            level,
            want,
            out[0].1.as_slice()
        );
    }
}

/// Guards the fix above against regression by sizing: the bookkeeping tables must
/// track as many levels as the consumer declared, so a level configured at the very
/// top of the range is still counted.
#[test]
fn every_configured_level_is_tracked_independently() {
    let mut s = server(1);

    // One failed attempt on each of the 12 levels. If the tables were still bounded
    // at 8, the last four would go unrecorded and report invalidKey forever; with
    // per-level tracking each one is independent and none escalates yet.
    for i in 0..12u8 {
        let level = i * 2 + 1;

        s.handle(&physical(&[0x27, level]), Instant::ZERO)
            .expect("seed request handled");
        let _ = drain(&mut s);

        s.handle(&physical(&[0x27, level + 1, 0x00]), Instant::ZERO)
            .expect("key request handled");
        let out = drain(&mut s);
        assert_eq!(out[0].1.as_slice(), &[0x7F, 0x27, 0x35]);
    }

    // Now the 12th level's second and third attempts must escalate, proving its count
    // was retained rather than dropped.
    for want in [0x35u8, 0x36] {
        let level = 11 * 2 + 1;
        s.handle(&physical(&[0x27, level]), Instant::ZERO)
            .expect("seed request handled");
        let _ = drain(&mut s);
        s.handle(&physical(&[0x27, level + 1, 0x00]), Instant::ZERO)
            .expect("key request handled");
        let out = drain(&mut s);
        assert_eq!(
            out[0].1[2], want,
            "level 0x{:02X} retained its count",
            level
        );
    }
}

// endregion: Security bookkeeping fails closed
