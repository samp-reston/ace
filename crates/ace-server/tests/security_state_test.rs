//! Security state that must not outlive the event that invalidates it.
//!
//! Three defects, each asserted on state or on the *distinction* between two negative
//! response codes rather than on the presence of a response, since the buggy and correct
//! implementations both answer — they answer differently.

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

// region: Fixture

const MAX_FRAME: usize = 32;
const MAX_OUTBOX: usize = 4;
const PHYSICAL: u16 = 0x1000;
const FUNCTIONAL: u16 = 0x7DF;
const TESTER: u32 = 0x0E80;

const LEVEL: u8 = 0x01;
const SEED_LENGTH: usize = 1;
const LOCKOUT_MS: u64 = 10_000;

/// NRCs these tests distinguish between.
const INVALID_KEY: u8 = 0x35;
const EXCEEDED_ATTEMPTS: u8 = 0x36;
const TIME_DELAY_NOT_EXPIRED: u8 = 0x37;
const REQUEST_SEQUENCE_ERROR: u8 = 0x24;
const CONDITIONS_NOT_CORRECT: u8 = 0x22;

struct Handler;

impl ServerHandler for Handler {
    type Error = BuiltinNrc;

    fn read_did(&self, _did: u16, buf: &mut [u8]) -> Result<usize, Self::Error> {
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
        Ok(())
    }
}

/// A provider whose key validation and seed length are both controllable.
struct Provider {
    accept_key: bool,
    seed_len: usize,
}

impl SecurityProvider for Provider {
    fn generate_seed(&mut self, _level: u8, buf: &mut [u8]) -> Result<usize, SecurityError> {
        if buf.len() < self.seed_len {
            return Err(SecurityError::InvalidKey);
        }

        for (i, b) in buf.iter_mut().take(self.seed_len).enumerate() {
            *b = 0xA0 | (i as u8);
        }

        Ok(self.seed_len)
    }

    fn validate_key(&self, _level: u8, _seed: &[u8], _key: &[u8]) -> Result<(), SecurityError> {
        if self.accept_key {
            Ok(())
        } else {
            Err(SecurityError::InvalidKey)
        }
    }
}

type TestServer = UdsServer<
    MAX_FRAME,
    MAX_OUTBOX,
    3,
    4,
    2,
    4,
    5_000,
    50,
    5_000,
    LOCKOUT_MS,
    3,
    4,
    2,
    Handler,
    Provider,
>;

fn server(accept_key: bool, seed_len: usize, max_attempts: u8) -> TestServer {
    let config = ServerConfig::new(PHYSICAL, FUNCTIONAL)
        .with_session(SessionConfig::default_session())
        .with_session(SessionConfig::extended_session())
        .with_service(ServiceConfig::new(0x27, &[0x01, 0x03]))
        .with_service(ServiceConfig::new(0x10, &[0x01, 0x03]))
        .with_service(ServiceConfig::new(0x11, &[0x01, 0x03]))
        .with_did(DidConfig::read_only(0xF190, &[0x01, 0x03]))
        .with_security_level(SecurityLevelConfig {
            level: LEVEL,
            max_attempts,
            lockout_duration: Duration::from_millis(LOCKOUT_MS),
            seed_length: SEED_LENGTH,
            key_length: 1,
        });

    UdsServer::new(
        config,
        Handler,
        Provider {
            accept_key,
            seed_len,
        },
        NodeAddress(u32::from(PHYSICAL)),
    )
}

/// Sends a request and returns the single response frame's bytes, if any.
fn exchange(s: &mut TestServer, data: &[u8]) -> Option<Vec<u8, MAX_FRAME>> {
    let frame = InboundFrame::new(NodeAddress(TESTER), NodeAddress(u32::from(PHYSICAL)), data);
    s.handle(&frame, Instant::ZERO).expect("handled");

    let mut out = Vec::new();
    s.drain_outbox(&mut out).expect("outbox fits");
    out.into_iter().next().map(|(_, frame)| frame)
}

/// The NRC byte of a negative response, or `None` if the response was positive.
fn nrc_of(frame: &Vec<u8, MAX_FRAME>) -> Option<u8> {
    if frame.first() == Some(&0x7F) {
        frame.get(2).copied()
    } else {
        None
    }
}

const REQUEST_SEED: [u8; 2] = [0x27, LEVEL];
const SEND_KEY: [u8; 3] = [0x27, LEVEL + 1, 0x00];

// endregion: Fixture

// region: A failed key invalidates the seed

/// After a rejected key the seed is gone, so a second key without a fresh `requestSeed`
/// is a sequence error rather than another attempt.
///
/// The distinction is the whole test. Both implementations answer; the buggy one answers
/// `invalidKey` again, because it validated the second key against the seed it had already
/// refused. That is the seed reuse the attempt counter is supposed to bound.
#[test]
fn a_rejected_key_invalidates_the_seed() {
    let mut s = server(false, SEED_LENGTH, 5);

    let seed = exchange(&mut s, &REQUEST_SEED).expect("seed response");
    assert_eq!(seed[0], 0x67, "positive requestSeed response");

    let first = exchange(&mut s, &SEND_KEY).expect("first key answered");
    assert_eq!(
        nrc_of(&first),
        Some(INVALID_KEY),
        "the first key is rejected on its merits"
    );

    let second = exchange(&mut s, &SEND_KEY).expect("second key answered");
    assert_eq!(
        nrc_of(&second),
        Some(REQUEST_SEQUENCE_ERROR),
        "the second key has no seed to be checked against and must be refused as \
         out of sequence, not validated again"
    );
}

/// The counterpart: a fresh seed makes a further attempt legitimate again.
#[test]
fn a_fresh_seed_permits_another_attempt() {
    let mut s = server(false, SEED_LENGTH, 5);

    exchange(&mut s, &REQUEST_SEED).expect("seed");
    exchange(&mut s, &SEND_KEY).expect("rejected");

    exchange(&mut s, &REQUEST_SEED).expect("fresh seed");
    let again = exchange(&mut s, &SEND_KEY).expect("answered");

    assert_eq!(
        nrc_of(&again),
        Some(INVALID_KEY),
        "with a seed in hand the key is judged again, not refused as out of sequence"
    );
}

// endregion: A failed key invalidates the seed

// region: ECU reset returns the server to its power-on state

/// A reset drops the session, the unlocked level, and any pending seed.
///
/// Asserted on state, because a reset that resets nothing still answers positively — the
/// response cannot tell the two apart.
#[test]
fn ecu_reset_returns_to_the_power_on_state() {
    let mut s = server(true, SEED_LENGTH, 5);

    // Enter a non-default session and unlock.
    exchange(&mut s, &[0x10, 0x03]).expect("session change");
    assert_eq!(s.session_type(), 0x03, "extended session entered");

    exchange(&mut s, &REQUEST_SEED).expect("seed");
    exchange(&mut s, &SEND_KEY).expect("key accepted");
    assert_eq!(s.security_level(), LEVEL, "level unlocked");

    // And leave a seed outstanding.
    exchange(&mut s, &REQUEST_SEED).expect("second seed");

    let reset = exchange(&mut s, &[0x11, 0x01]).expect("reset answered");
    assert_eq!(reset[0], 0x51, "0x11 + 0x40");

    assert_eq!(s.session_type(), 0x01, "back to the default session");
    assert_eq!(
        s.security_level(),
        0,
        "an unlocked level must not outlive the reset"
    );

    let after = exchange(&mut s, &SEND_KEY).expect("answered");
    assert_eq!(
        nrc_of(&after),
        Some(REQUEST_SEQUENCE_ERROR),
        "and neither may a pending seed"
    );
}

/// A lockout *does* survive a reset.
///
/// Otherwise a reset is an escape from the attempt limit: an attacker resets between
/// guesses and is never delayed, which defeats the mechanism entirely.
#[test]
fn a_lockout_survives_an_ecu_reset() {
    let mut s = server(false, SEED_LENGTH, 1);

    exchange(&mut s, &REQUEST_SEED).expect("seed");
    let locked = exchange(&mut s, &SEND_KEY).expect("answered");
    assert_eq!(
        nrc_of(&locked),
        Some(EXCEEDED_ATTEMPTS),
        "one attempt with max_attempts=1 triggers the lockout"
    );

    exchange(&mut s, &[0x11, 0x01]).expect("reset answered");

    let after = exchange(&mut s, &REQUEST_SEED).expect("answered");
    assert_eq!(
        nrc_of(&after),
        Some(TIME_DELAY_NOT_EXPIRED),
        "the delay must still be in force after a reset"
    );
}

// endregion: ECU reset returns the server to its power-on state

// region: The configured key length is enforced

/// A malformed key length is rejected as invalid format and does not consume the pending seed.
#[test]
fn a_key_of_the_wrong_length_is_refused_without_consuming_the_seed() {
    let mut s = server(true, SEED_LENGTH, 5);

    exchange(&mut s, &REQUEST_SEED).expect("seed response");

    // The configured key length is one byte, but this request carries no key bytes.
    let malformed = exchange(&mut s, &[0x27, LEVEL + 1]).expect("answered");
    assert_eq!(
        nrc_of(&malformed),
        Some(0x13),
        "wrong key length is malformed input, not an invalid key"
    );

    let valid = exchange(&mut s, &SEND_KEY).expect("valid key answered");
    assert_eq!(
        valid[0], 0x67,
        "the malformed request must not consume the seed"
    );
}

// endregion: The configured key length is enforced

// region: The configured seed length is enforced

/// A provider returning a different seed length than the level declares is refused.
///
/// Shipping it would have the tester derive a key against a seed of unexpected length and
/// fail the unlock with nothing to explain why.
#[test]
fn a_seed_of_the_wrong_length_is_refused() {
    // The level declares 1 byte; the provider returns 2.
    let mut s = server(true, SEED_LENGTH + 1, 5);

    let response = exchange(&mut s, &REQUEST_SEED).expect("answered");
    assert_eq!(
        nrc_of(&response),
        Some(CONDITIONS_NOT_CORRECT),
        "the server's own contract violation, not a malformed request"
    );

    let after = exchange(&mut s, &SEND_KEY).expect("answered");
    assert_eq!(
        nrc_of(&after),
        Some(REQUEST_SEQUENCE_ERROR),
        "and no seed is left pending from the refused request"
    );
}

/// The control: the declared length is accepted.
#[test]
fn a_seed_of_the_declared_length_is_accepted() {
    let mut s = server(true, SEED_LENGTH, 5);

    let response = exchange(&mut s, &REQUEST_SEED).expect("answered");
    assert_eq!(response[0], 0x67, "positive requestSeed response");
    assert_eq!(
        response.len(),
        2 + SEED_LENGTH,
        "response SID, level, then the seed"
    );
}

// endregion: The configured seed length is enforced
