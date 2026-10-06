//! Generic, protocol-neutral scripted fault injection.

use ace_core::clock::Duration;
use ace_core::io::NodeAddress;
use ace_sim::bus::SimBus;
use ace_sim::fault::{FaultConfig, FaultPlan, ScriptedFault};

const DATA: usize = 8;
const QUEUED: usize = 4;
const A: NodeAddress = NodeAddress(1);
const B: NodeAddress = NodeAddress(2);

fn plan(actions: &[ScriptedFault]) -> FaultPlan {
    let mut plan = FaultPlan::new();
    for action in actions {
        plan.push(*action).expect("test plan fits");
    }
    plan
}

fn faults(script: FaultPlan) -> FaultConfig {
    let mut faults = FaultConfig::none();
    faults.script = script;
    faults
}

#[test]
fn scripted_drop_suppresses_the_next_send_only() {
    let mut bus: SimBus<DATA, QUEUED> = SimBus::new(1, faults(plan(&[ScriptedFault::DropNext])));

    assert!(!bus.send(A.clone(), B.clone(), &[1]));
    assert!(bus.send(A.clone(), B.clone(), &[2]));

    let delivered = bus.tick(Duration::ZERO);
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].data.as_slice(), &[2]);
}

#[test]
fn scripted_delay_is_exact_and_deterministic() {
    let mut bus: SimBus<DATA, QUEUED> = SimBus::new(
        2,
        faults(plan(&[ScriptedFault::DelayNext(Duration::from_micros(10))])),
    );

    assert!(bus.send(A.clone(), B.clone(), &[3]));
    assert!(bus.tick(Duration::from_micros(9)).is_empty());
    assert_eq!(bus.tick(Duration::from_micros(1)).len(), 1);
}

#[test]
fn scripted_duplicate_delivers_two_identical_envelopes() {
    let mut bus: SimBus<DATA, QUEUED> =
        SimBus::new(3, faults(plan(&[ScriptedFault::DuplicateNext])));

    assert!(bus.send(A.clone(), B.clone(), &[4]));
    let delivered = bus.tick(Duration::ZERO);

    assert_eq!(delivered.len(), 2);
    assert_eq!(delivered[0], delivered[1]);
}

#[test]
fn scripted_byte_replacement_is_opaque_to_the_bus() {
    let mut bus: SimBus<DATA, QUEUED> = SimBus::new(
        4,
        faults(plan(&[ScriptedFault::ReplaceByte {
            offset: 1,
            value: 0x7F,
        }])),
    );

    assert!(bus.send(A.clone(), B.clone(), &[0x10, 0x00, 0x03]));
    let delivered = bus.tick(Duration::ZERO);

    assert_eq!(delivered[0].data.as_slice(), &[0x10, 0x7F, 0x03]);
}

#[test]
fn duplicate_requires_two_queue_slots_or_sends_nothing() {
    let mut bus: SimBus<DATA, 1> = SimBus::new(5, faults(plan(&[ScriptedFault::DuplicateNext])));

    assert!(!bus.send(A.clone(), B.clone(), &[5]));
    assert!(bus.tick(Duration::ZERO).is_empty());
}
