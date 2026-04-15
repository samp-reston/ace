// region: Imports

use ace_client::event::ClientEvent;
use ace_sim::clock::Duration;
use ace_sim::fault::FaultConfig;

use crate::{
    dst::{MAX_TICKS, TICK_MS},
    harness::{expect_periodic, expect_positive, DstScenario},
};

// endregion: Imports

// region: P1 - periodic data arrives after subscription

#[test]
fn p1_periodic_data_arrives_at_fast_rate() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    // Subscribe client to DID low byte 0x90 (DID 0xF290)
    s.client.subscribe_periodic(0x90);

    // Request fast rate (0x03) for periodic identifier 0x90
    s.client
        .request(&[0x2A, 0x03, 0x90], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x2A);

    // Advance past one fast rate interval (50ms) + margin
    s.tick_n(100, TICK_MS);

    let data = expect_periodic(&mut s.client, 0x90);
    assert!(!data.is_empty(), "periodic data should not be empty");
}

// endregion: P1

// region: P2 - stop sending cancels periodic

#[test]
fn p2_stop_sending_cancels_periodic() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    s.client.subscribe_periodic(0x90);

    // Start fast rate
    s.client
        .request(&[0x2A, 0x03, 0x90], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x2A);

    // Stop sending
    s.client
        .request(&[0x2A, 0x04, 0x90], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x2A);

    // Drain any events already in the queue
    s.client.drain_events().count();

    // Advance well past fast rate interval - no new periodic events should arrive
    s.tick_n(500, TICK_MS);

    let periodic_count = s
        .client
        .drain_events()
        .filter(|e| matches!(e, ClientEvent::PeriodicData { did: 0x90, .. }))
        .count();

    assert_eq!(periodic_count, 0, "expected no periodic events after stop");
}

// endregion: P2

// region: P3 - periodic data carries correct DID and content

#[test]
fn p3_periodic_data_correct_did_and_content() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    s.client.subscribe_periodic(0x90);

    s.client
        .request(&[0x2A, 0x03, 0x90], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);
    expect_positive(&mut s.client, 0x2A);

    s.tick_n(200, TICK_MS);

    let data = expect_periodic(&mut s.client, 0x90);
    // VIN from fixture: "TESTVIN1234567890"
    assert_eq!(
        data.as_slice(),
        b"TESTVIN1234567890",
        "periodic data content mismatch"
    );
}

// endregion: P3
