use ace_core::clock::Duration;
use ace_core::io::NodeAddress;
use ace_sim::fault::{FaultConfig, FaultPlan, ScriptedFault};
use ace_sim::trace::TraceBus;

fn main() {
    let sender = NodeAddress(1);
    let receiver = NodeAddress(2);

    let mut plan = FaultPlan::new();
    plan.push(ScriptedFault::DelayNext(Duration::from_micros(10)))
        .expect("script capacity");

    let mut faults = FaultConfig::none();
    faults.script = plan;

    let mut bus: TraceBus<8, 4, 16> = TraceBus::new(7, faults);
    assert!(bus
        .send(sender.clone(), receiver.clone(), &[0x10, 0x03])
        .expect("trace capacity"));

    assert!(bus
        .tick(Duration::from_micros(9))
        .expect("trace capacity")
        .is_empty());
    let recorded_delivery = bus.tick(Duration::from_micros(1)).expect("trace capacity");
    assert_eq!(recorded_delivery.len(), 1);

    let trace = bus.trace().clone();
    let mut replay = trace.replay();
    assert!(replay
        .send(&sender, &receiver, &[0x10, 0x03], Duration::ZERO)
        .expect("recorded send"));
    assert!(replay
        .tick(Duration::from_micros(9))
        .expect("recorded tick")
        .is_empty());
    assert_eq!(
        replay.tick(Duration::from_micros(1)).expect("recorded tick"),
        recorded_delivery
    );
    replay.finish().expect("complete trace");
}
