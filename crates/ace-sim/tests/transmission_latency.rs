//! A sent message is not delivered before its transmission time has elapsed.
//!
//! `SimBus` previously scheduled every message for the send instant itself, so a message sent
//! and a subsequent `tick()` call within the same simulation step were indistinguishable from
//! instantaneous delivery - the loop's send/tick ordering was the only thing deciding whether
//! a hop cost one step or two. These tests exist because the DST suite's own tick and payload
//! sizes are too coarse to exercise the mechanism: its ticks (100us CAN, 1ms TCP) and payloads
//! dwarf the microsecond-scale transmission times realistic bit rates produce, so the DST
//! passing unchanged after this change proves nothing about whether delivery is actually
//! delayed - only that nothing here was large or slow enough to notice either way.

use ace_core::clock::Duration;
use ace_core::io::NodeAddress;
use ace_sim::can_bus::{CanFaultConfig, CanSimBus};
use ace_sim::tcp_bus::{TcpFaultConfig, TcpSimBus};

const A: NodeAddress = NodeAddress(1);
const B: NodeAddress = NodeAddress(2);

// region: CAN

/// A frame is not delivered on the very tick it was sent, when the bus is slow enough that its
/// transmission time exceeds the tick duration.
#[test]
fn a_can_frame_is_not_delivered_within_its_own_transmission_time() {
    let mut faults = CanFaultConfig::none();
    // Slow enough that an 8-byte frame's ~944us transmission time exceeds a 100us tick.
    faults.bit_rate = 1_000;

    let mut bus: CanSimBus<8, 4> = CanSimBus::new(1, faults);
    assert!(bus.send(A, B, &[0u8; 8]));

    let delivered = bus.tick(Duration::from_micros(100));
    assert!(
        delivered.is_empty(),
        "a frame taking ~944us to transmit must not arrive after a 100us tick"
    );
}

/// The same frame arrives once enough simulated time has passed for its transmission time to
/// elapse - proving the delay is bounded, not permanent.
#[test]
fn a_can_frame_is_delivered_once_its_transmission_time_elapses() {
    let mut faults = CanFaultConfig::none();
    faults.bit_rate = 1_000;

    let mut bus: CanSimBus<8, 4> = CanSimBus::new(1, faults);
    assert!(bus.send(A, B, &[0u8; 8]));

    bus.tick(Duration::from_micros(100));
    let delivered = bus.tick(Duration::from_millis(200));

    assert_eq!(
        delivered.len(),
        1,
        "the frame must arrive once its transmission time has passed"
    );
}

/// At a realistic 500kbit/s bus, an 8-byte frame's transmission time is well under a
/// millisecond - the control proving the defaults do not make every test glacial.
#[test]
fn a_can_frame_at_the_default_bit_rate_arrives_within_a_millisecond() {
    let mut bus: CanSimBus<8, 4> = CanSimBus::new(1, CanFaultConfig::none());
    assert!(bus.send(A, B, &[0u8; 8]));

    let delivered = bus.tick(Duration::from_millis(1));
    assert_eq!(delivered.len(), 1);
}

// endregion: CAN

// region: TCP

/// A segment is not delivered within its own transmission time, mirroring the CAN case for the
/// connection-oriented bus.
#[test]
fn a_tcp_segment_is_not_delivered_within_its_own_transmission_time() {
    let mut faults = TcpFaultConfig::none();
    // Slow enough that a 12-byte segment's ~53us transmission time exceeds a 10us tick.
    faults.bit_rate = 10_000;

    let mut bus: TcpSimBus<32, 4, 4, 2> = TcpSimBus::new(1, faults);
    bus.connect(A, B);
    bus.tick(Duration::ZERO);

    assert!(bus.send(A, B, &[0u8; 12]));

    let delivered = bus.tick(Duration::from_micros(10));
    assert!(
        delivered.is_empty(),
        "a segment taking ~53us to transmit must not arrive after a 10us tick"
    );
}

/// At the default 100Mbit/s link rate, a small DoIP segment arrives within a millisecond - the
/// control proving realistic defaults do not slow the common case down.
#[test]
fn a_tcp_segment_at_the_default_bit_rate_arrives_within_a_millisecond() {
    let mut bus: TcpSimBus<32, 4, 4, 2> = TcpSimBus::new(1, TcpFaultConfig::none());
    bus.connect(A, B);
    bus.tick(Duration::ZERO);

    assert!(bus.send(A, B, &[0u8; 12]));

    let delivered = bus.tick(Duration::from_millis(1));
    assert_eq!(delivered.len(), 1);
}

// endregion: TCP
