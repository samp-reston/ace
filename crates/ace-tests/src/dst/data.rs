// region: Imports

use crate::dst::{MAX_TICKS, TICK_MS};
use crate::harness::{assert_session, expect_nrc, expect_positive, DstScenario, SEEDS_BASELINE};
use ace_sim::clock::Duration;
use ace_sim::fault::FaultConfig;

// endregion: Imports

// region: P1 - RDBI returns expected data

#[test]
fn p1_rdbi_vin_default_session() {
    for seed in SEEDS_BASELINE {
        let mut s = DstScenario::new(seed, FaultConfig::none());

        // 0xF190 is readable in all sessions
        s.client
            .request(&[0x22, 0xF1, 0x90], s.runner.bus().now())
            .unwrap();
        s.tick_until_quiet(TICK_MS, MAX_TICKS);

        let data = expect_positive(&mut s.client, 0x22);
        // Response: [DID_high, DID_low, data...]
        assert_eq!(data.get(0).copied(), Some(0xF1), "seed {seed}");
        assert_eq!(data.get(1).copied(), Some(0x90), "seed {seed}");
        // VIN from fixture: "TESTVIN1234567890"
        let vin = data.get(2..).unwrap_or(&[]);
        assert_eq!(vin, b"TESTVIN1234567890", "seed {seed}: VIN mismatch");
    }
}

// endregion: P1

// region: P2 - unknown DID returns NRC

#[test]
fn p2_rdbi_unknown_did_returns_nrc() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    // 0xDEAD is not configured in the test server
    s.client
        .request(&[0x22, 0xDE, 0xAD], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let nrc = expect_nrc(&mut s.client, 0x22);
    assert_eq!(
        nrc, 0x31,
        "expected RequestOutOfRange 0x31, got 0x{:02X}",
        nrc
    );
}

// endregion: P2

// region: P3 - DID not readable in current session

#[test]
fn p3_rdbi_not_readable_in_session_returns_nrc() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    // 0xF101 is only readable in extended and programming sessions
    // Default session should return requestOutOfRange
    s.client
        .request(&[0x22, 0xF1, 0x01], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let nrc = expect_nrc(&mut s.client, 0x22);
    assert_eq!(
        nrc, 0x31,
        "expected RequestOutOfRange 0x31, got 0x{:02X}",
        nrc
    );
}

// endregion: P3

// region: P4 - WDBI round trip

#[test]
fn p4_wdbi_round_trip() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    // Must be in extended session with security unlocked to write 0xF120
    s.client
        .request(&[0x10, 0x03], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x10);

    // Unlock security level 1
    s.client
        .request(&[0x27, 0x01], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    let seed_resp = expect_positive(&mut s.client, 0x27);
    let seed_byte = seed_resp.get(1).copied().unwrap_or(0);
    let key = seed_byte ^ 0xFF;
    s.client
        .request(&[0x27, 0x02, key], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x27);

    // Write new value to 0xF120
    s.client
        .request(&[0x2E, 0xF1, 0x20, 0xAA, 0xBB], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x2E);

    // Read back and verify
    s.client
        .request(&[0x22, 0xF1, 0x20], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    let data = expect_positive(&mut s.client, 0x22);
    let value = data.get(2..).unwrap_or(&[]);
    assert_eq!(value, &[0xAA, 0xBB], "written value not reflected in read");
}

// endregion: P4

// region: P5 - WDBI requires security

#[test]
fn p5_wdbi_without_security_returns_nrc() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    // Extended session but no security unlock
    s.client
        .request(&[0x10, 0x03], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x10);

    s.client
        .request(&[0x2E, 0xF1, 0x20, 0xAA, 0xBB], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let nrc = expect_nrc(&mut s.client, 0x2E);
    assert_eq!(
        nrc, 0x33,
        "expected SecurityAccessDenied 0x33, got 0x{:02X}",
        nrc
    );
}

// endregion: P5
