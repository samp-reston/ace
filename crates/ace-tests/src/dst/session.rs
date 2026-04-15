// region: Imports

use ace_client::event::ClientEvent;
use ace_sim::{clock::Duration, fault::FaultConfig};
use heapless::Vec;

use crate::{
    dst::{MAX_TICKS, TICK_MS},
    harness::{
        assert_session, expect_nrc, expect_positive, DstScenario, SEEDS_BASELINE, SEEDS_CHAOS,
        SEEDS_LIGHT,
    },
};

// endregion: Imports

// region: P1 - valid session transition, no faults

#[test]
fn p1_session_control_extended_no_faults() {
    for seed in SEEDS_BASELINE {
        let mut s = DstScenario::new(seed, FaultConfig::none());

        // Request: [SID 0x10, session type 0x03 extended]
        s.client
            .request(&[0x10, 0x03], s.runner.bus().now())
            .expect("request should not fail");

        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let data = expect_positive(&mut s.client, 0x10);

        // Positive response payload: [session_type, p2_high, p2_low, p2ext_high, p2ext_low]
        assert_eq!(
            data.first().copied(),
            Some(0x03),
            "seed {seed}: response session type mismatch"
        );

        assert_session(&s.server, 0x03);
    }
}

#[test]
fn p1_session_control_programming_no_faults() {
    for seed in SEEDS_BASELINE {
        let mut s = DstScenario::new(seed, FaultConfig::none());

        s.client
            .request(&[0x10, 0x02], s.runner.bus().now())
            .expect("request should not fail");

        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let data = expect_positive(&mut s.client, 0x10);

        assert_eq!(data.first().copied(), Some(0x02), "seed {seed}");

        assert_session(&s.server, 0x02);
    }
}

#[test]
fn p1_session_control_default_no_faults() {
    for seed in SEEDS_BASELINE {
        let mut s = DstScenario::new(seed, FaultConfig::none());

        // Transition to extended first, then back to default
        s.client
            .request(&[0x10, 0x03], s.runner.bus().now())
            .expect("request should not fail");
        s.tick_until_quiet(TICK_MS, MAX_TICKS);
        expect_positive(&mut s.client, 0x10);

        s.client
            .request(&[0x10, 0x01], s.runner.bus().now())
            .expect("request should not fail");
        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let data = expect_positive(&mut s.client, 0x10);
        assert_eq!(data.first().copied(), Some(0x01), "seed {seed}");
        assert_session(&s.server, 0x01);
    }
}

// endregion: P1

// region: P2 - S3 timeout returns server to default session

#[test]
fn p2_s3_timeout_returns_to_default() {
    // Use no faults so S3 timing is deterministic
    let mut s = DstScenario::new(0, FaultConfig::none());

    // Transition to extended session
    let _ = s.client.request(&[0x10, 0x03], s.runner.bus().now());

    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    expect_positive(&mut s.client, 0x10);
    assert_session(&s.server, 0x03);

    // Advance past S3 timeout (default 5000ms) without sending TesterPresent
    s.tick_n(6_000, TICK_MS);

    assert_session(&s.server, 0x01);
}

// endregion: P2

// region: P3 - unsupported session returns NRC

#[test]
fn p3_unsupported_session_returns_nrc() {
    for seed in SEEDS_BASELINE {
        let mut s = DstScenario::new(seed, FaultConfig::none());

        // Session 0x7F is not configured in the test server
        s.client
            .request(&[0x10, 0x7F], s.runner.bus().now())
            .expect("request should not fail");

        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let nrc = expect_nrc(&mut s.client, 0x10);

        assert_eq!(
            nrc, 0x12,
            "seed {seed}: expected SubFunctionNotSupported 0x12, got 0x{:02X}",
            nrc
        );

        // Server must remain in default session
        assert_session(&s.server, 0x01);
    }
}

// endregion: P3

// region: P4 - properties hold under fault injection

#[test]
fn p4_session_control_light_faults() {
    for seed in SEEDS_LIGHT {
        let mut s = DstScenario::new(seed, FaultConfig::light());

        s.client
            .request(&[0x10, 0x03], s.runner.bus().now())
            .expect("request should not fail");

        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        // Under light faults the exchange must complete - either positive response (message
        // delivered) or timeout (message lost). Neither silence nor panic is acceptable.
        let events: Vec<_, 32> = s.client.drain_events().collect();

        let resolved = events.iter().any(|e| {
            matches!(
                e,
                ClientEvent::PositiveResponse { sid: 0x10, .. }
                    | ClientEvent::NegativeResponse { sid: 0x10, .. }
                    | ClientEvent::Timeout { sid: 0x10 }
            )
        });
        assert!(
            resolved,
            "seed {seed}: exchange did not resolve under light faults"
        );
    }
}

#[test]
fn p4_session_control_chaos_faults() {
    for seed in SEEDS_CHAOS {
        let mut s = DstScenario::new(seed, FaultConfig::chaos());

        s.client
            .request(&[0x10, 0x03], s.runner.bus().now())
            .expect("request should not fail");

        // Give more ticks under chaos - delays can be significant
        s.tick_n(MAX_TICKS * 10, TICK_MS);

        // Property: the exchange must have resolved - never left pending indefinitely. Client may
        // report timeout bus must not be silent.
        let events: Vec<_, 32> = s.client.drain_events().collect();

        let resolved = events.iter().any(|e| {
            matches!(
                e,
                ClientEvent::PositiveResponse { sid: 0x10, .. }
                    | ClientEvent::NegativeResponse { sid: 0x10, .. }
                    | ClientEvent::Timeout { sid: 0x10 }
            )
        }) || s.client.pending_count() == 0;

        assert!(
            resolved,
            "seed {seed}: client still has pending request after chaos run"
        );
    }
}
// endregion: P4
