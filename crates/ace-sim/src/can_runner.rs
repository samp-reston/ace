//! CanSimRunner - drives SimNodeErased slices over a CanSimBus.
//!
//! Extends the SimRunner concept for CAN-aware scenarios. Alongside message delivery it:
//!     - Delivers CanEvents to nodes that implement CanEventHandler
//!     - Suppresses message delivery when the bus is in bus-off state
//!     - Drains node outboxes back onto the CAN bus

// region: Imports

use crate::{
    can_bus::{CAN_MAX_EVENTS, CanEvent, CanSimBus},
    clock::{Duration, Instant},
    io::{InboundFrame, NodeAddress},
    node::SimNodeErased,
};
use ace_core::Vec;

// endregion: Imports

// region: CanEventHandler

/// Optional trait for nodes that need to observe CAN bus events.
///
/// Nodes may implement this to react to bus-off, recovery, and bit errors. The `CanSimRunner`
/// calls this after message delivery on each tick.
pub trait CanEventHandler {
    fn on_can_event(&mut self, event: &CanEvent, now: Instant);
}

// endregion: CanEventHandler

// region: CanSimRunner

/// Drivers `SimNodeErased` slices over a `CanSimBus`.
///
/// `N` - max CAN frame payload bytes (8 classic, 64 FD)
/// `Q` - max frames in-flight on the bus
pub struct CanSimRunner<const N: usize, const Q: usize> {
    bus: CanSimBus<N, Q>,
}

impl<const MAX_FRAME: usize, const MAX_OUTBOX: usize> CanSimRunner<MAX_FRAME, MAX_OUTBOX> {
    pub fn new(bus: CanSimBus<MAX_FRAME, MAX_OUTBOX>) -> Self {
        Self { bus }
    }

    pub fn bus(&mut self) -> &mut CanSimBus<MAX_FRAME, MAX_OUTBOX> {
        &mut self.bus
    }

    pub fn now(&self) -> Instant {
        self.bus.now()
    }

    /// Ticks the simulation by `duration`.
    ///
    /// Order of operations per tick:
    ///     1. Advance bus clock, apply CAN fault injection, deliver due frames (no frames
    ///        delivered if bus is in bus-off state)
    ///     2. Deliver CAN events to nodes implementing `CanEventHandler`
    ///     3. Tick all nodes
    ///     4. Drain node outboxes onto the bus
    ///
    /// Returns the number of frames delivered.
    pub fn tick(
        &mut self,
        nodes: &mut [&mut dyn SimNodeErased<MAX_FRAME, MAX_OUTBOX>],
        can_event_nodes: &mut [&mut dyn CanEventHandler],
        duration: Duration,
    ) -> usize {
        let delivered = self.bus.tick(duration);
        let now = self.bus.now();
        let mut count = 0;

        for envelope in &delivered {
            // Counts frames put on the segment, not node deliveries: each frame
            // below is offered to every node, so counting per node would inflate
            // this by the node count and contradict what it documents.
            count += 1;

            for node in nodes.iter_mut() {
                // Offered to every node on the segment, as a real CAN or Ethernet
                // link does. Filtering by destination here made the harness do the
                // nodes' job, so a node that cannot filter its own traffic looked
                // correct in simulation and would answer everything in production.
                let frame =
                    InboundFrame::new(envelope.src.clone(), envelope.dst.clone(), &envelope.data);
                node.handle(&frame, now);
            }
        }

        // Sized to the bus queue, so this cannot overflow. Failing loudly on a
        // violated invariant is deliberate: silently truncating events would
        // change what a deterministic run observes, with no visible signal.
        let can_events: Vec<CanEvent, CAN_MAX_EVENTS> =
            Vec::from_iter_checked(self.bus.drain_events())
                .expect("event buffer is sized to the bus queue");
        for event in &can_events {
            for handler in can_event_nodes.iter_mut() {
                handler.on_can_event(event, now);
            }
        }

        for node in nodes.iter_mut() {
            node.tick(now);
        }

        let mut outbox: Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX> = Vec::new();
        for node in nodes.iter_mut() {
            outbox.clear();
            node.drain_outbox(&mut outbox);

            let src = node.address().clone();
            for (dst, data) in outbox.iter() {
                self.bus.send(src.clone(), dst.clone(), data);
            }
        }

        count
    }
}

// endregion: CanSimRunner
