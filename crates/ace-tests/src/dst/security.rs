// region: Imports

use ace_sim::clock::Duration;
use ace_sim::fault::FaultConfig;

use crate::{
    dst::{MAX_TICKS, TICK_MS},
    harness::{
        assert_security, assert_session, expect_nrc, expect_positive, DstScenario, ECU_ADDR,
        SEEDS_BASELINE, TESTER_ADDR,
    },
};

// endregion: Imports

// TestSecurityProvider: seed = level byte, key = seed XOR 0xFF
// For level 0x01: seed = 0x01, key = 0xFE

// region: P1 - valid seed/key exchange

#[test]
fn p1_security_access_level1_unlocks() {
    for seed in SEEDS_BASELINE {
        let mut s = DstScenario::new(seed, FaultConfig::none());

        // Must be in extended session to access security
        s.client
            .request(&[0x10, 0x03], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);
        expect_positive(&mut s.client, 0x10);

        // RequestSeed for level 0x01
        s.client
            .request(&[0x27, 0x01], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let seed_resp = expect_positive(&mut s.client, 0x27);
        // Response: [level_byte, seed_byte]
        let level = seed_resp.first().copied().unwrap_or(0);
        let seed_byte = seed_resp.get(1).copied().unwrap_or(0);
        assert_eq!(
            level, 0x01,
            "seed {seed}: unexpected level in seed response"
        );

        // SendKey: key = seed XOR 0xFF
        let key = seed_byte ^ 0xFF;
        s.client
            .request(&[0x27, 0x02, key], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        expect_positive(&mut s.client, 0x27);
        assert_security(&s.server, 0x01);
    }
}

// endregion: P1

// region: P2 - wrong key increments failed attempts

#[test]
fn p2_wrong_key_returns_invalid_key_nrc() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    s.client
        .request(&[0x10, 0x03], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x10);

    s.client
        .request(&[0x27, 0x01], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x27);

    // Send wrong key
    s.client
        .request(&[0x27, 0x02, 0x00], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let nrc = expect_nrc(&mut s.client, 0x27);
    assert_eq!(nrc, 0x35, "expected InvalidKey 0x35, got 0x{:02X}", nrc);
    assert_security(&s.server, 0);
}

// endregion: P2

// region: P3 - lockout after max attempts

#[test]
fn p3_lockout_after_max_attempts() {
    let mut s = DstScenario::new(0, FaultConfig::chaos());

    s.client
        .request(&[0x10, 0x03], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x10);

    // Exhaust all 3 attempts with wrong keys
    for attempt in 0..3u8 {
        s.client
            .request(&[0x27, 0x01], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);
        expect_positive(&mut s.client, 0x27);

        s.client
            .request(&[0x27, 0x02, 0x00], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let nrc = expect_nrc(&mut s.client, 0x27);
        if attempt < 2 {
            assert_eq!(nrc, 0x35, "attempt {attempt}: expected invalidKey");
        } else {
            assert_eq!(
                nrc, 0x36,
                "attempt {attempt}: expected exceededNumberOfAttempts"
            );
        }
    }

    // Further RequestSeed must return requiredTimeDelayNotExpired
    s.client
        .request(&[0x27, 0x01], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let nrc = expect_nrc(&mut s.client, 0x27);
    assert_eq!(
        nrc, 0x37,
        "expected requiredTimeDelayNotExpired 0x37, got 0x{:02X}",
        nrc
    );
}

// endregion: P3

// region: P4 - lockout expires

#[test]
fn p4_lockout_expires_after_duration() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    s.client
        .request(&[0x10, 0x03], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x10);

    // Exhaust attempts to trigger lockout
    for _ in 0..3u8 {
        s.client
            .request(&[0x27, 0x01], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);
        expect_positive(&mut s.client, 0x27);

        s.client
            .request(&[0x27, 0x02, 0x00], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);
        s.client.drain_events().count(); // discard
    }

    // Advance past lockout duration (10_000ms)
    s.tick_n(11_000, TICK_MS);

    // RequestSeed should now succeed
    s.client
        .request(&[0x27, 0x01], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let resp = expect_positive(&mut s.client, 0x27);
    let seed_byte = resp.get(1).copied().unwrap_or(0);
    let key = seed_byte ^ 0xFF;

    s.client
        .request(&[0x27, 0x02, key], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    expect_positive(&mut s.client, 0x27);
    assert_security(&s.server, 0x01);
}

// endregion: P4
