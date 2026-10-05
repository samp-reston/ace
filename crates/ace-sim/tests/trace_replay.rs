//! Deterministic trace recording and replay.

use ace_core::clock::{Duration, Instant};
use ace_core::io::NodeAddress;
use ace_sim::fault::FaultConfig;
use ace_sim::trace::{ReplayError, TraceBus, TraceError};

const DATA: usize = 8;
const QUEUED: usize = 4;
const STEPS: usize = 8;

const A: NodeAddress = NodeAddress(1);
const B: NodeAddress = NodeAddress(2);

#[test]
fn replay_returns_the_recorded_delivery_not_a_new_random_run() {
    let mut bus: TraceBus<DATA, QUEUED, STEPS> = TraceBus::new(0x51, FaultConfig::none());

    assert!(
        bus.send_with_min_latency(A.clone(), B.clone(), &[1, 2, 3], Duration::from_micros(50))
            .expect("trace has capacity")
    );

    let early = bus
        .tick(Duration::from_micros(49))
        .expect("trace has capacity");
    assert!(early.is_empty());

    let recorded_delivery = bus
        .tick(Duration::from_micros(1))
        .expect("trace has capacity");
    assert_eq!(recorded_delivery.len(), 1);
    assert_eq!(bus.now(), Instant::from_micros(50));

    let trace = bus.trace().clone();
    let mut replay = trace.replay();

    assert!(
        replay
            .send(&A, &B, &[1, 2, 3], Duration::from_micros(50))
            .expect("recorded send exists")
    );
    assert!(
        replay
            .tick(Duration::from_micros(49))
            .expect("recorded tick exists")
            .is_empty()
    );

    let replayed_delivery = replay
        .tick(Duration::from_micros(1))
        .expect("recorded tick exists");
    assert_eq!(replayed_delivery, recorded_delivery);
    assert_eq!(replay.now(), Instant::from_micros(50));
    replay.finish().expect("the whole trace was consumed");
}

#[test]
fn replay_preserves_a_fault_dropped_send() {
    let mut faults = FaultConfig::none();
    faults.message_loss = (1, 1);

    let mut bus: TraceBus<DATA, QUEUED, STEPS> = TraceBus::new(0x52, faults);
    assert!(
        !bus.send(A.clone(), B.clone(), &[9])
            .expect("trace has capacity")
    );

    let trace = bus.trace().clone();
    let mut replay = trace.replay();
    assert!(
        !replay
            .send(&A, &B, &[9], Duration::ZERO)
            .expect("recorded send exists")
    );
    replay.finish().expect("the whole trace was consumed");
}

#[test]
fn replay_rejects_a_different_operation() {
    let mut bus: TraceBus<DATA, QUEUED, STEPS> = TraceBus::new(0x53, FaultConfig::none());
    bus.send(A.clone(), B.clone(), &[1])
        .expect("trace has capacity");

    let trace = bus.trace().clone();
    let mut replay = trace.replay();
    assert_eq!(
        replay.send(&A, &B, &[2], Duration::ZERO),
        Err(ReplayError::SendMismatch)
    );
    assert_eq!(replay.remaining(), 1);
}

#[test]
fn trace_capacity_is_reported_before_the_underlying_bus_advances() {
    let mut bus: TraceBus<DATA, QUEUED, 1> = TraceBus::new(0x54, FaultConfig::none());

    bus.send(A.clone(), B.clone(), &[1])
        .expect("first step fits");
    assert_eq!(
        bus.tick(Duration::ZERO),
        Err(TraceError::Full),
        "the full trace must reject the operation rather than mutate the bus unrecorded"
    );
}
