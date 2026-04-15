//! CanSimRunner - drives SimNodeErased slices over a CanSimBus.
//!
//! Extends the SimRunner concept for CAN-aware scenarios. Alongside message delivery it:
//!     - Delivers CanEvents to nodes that implement CanEventHandler
//!     - Suppresses message delivery when the bus is in bus-off state
//!     - Drains node outboxes back onto the CAN bus

// region: Imports

use crate::{
    can_bus::{CanEvent, CanSimBus},
    clock::{Duration, Instant},
    io::NodeAddress,
    node::SimNodeErased,
};

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

impl<const N: usize, const Q: usize> CanSimRunner<N, Q> {
    pub fn new(bus: CanSimBus<N, Q>) -> Self {
        Self { bus }
    }

    pub fn bus(&mut self) -> &mut CanSimBus<N, Q> {
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
        nodes: &mut [&mut dyn SimNodeErased<N, Q>],
        can_event_nodes: &mut [&mut dyn CanEventHandler],
        duration: Duration,
    ) -> usize {
        let delivered = self.bus.tick(duration);
        let now = self.bus.now();
        let mut count = 0;

        for envelope in &delivered {
            for node in nodes.iter_mut() {
                if *node.address() == envelope.dst {
                    node.handle(&envelope.src, &envelope.data, now);
                    count += 1;
                }
            }
        }

        let can_events: heapless::Vec<CanEvent, 16> = self.bus.drain_events().collect();
        for event in &can_events {
            for handler in can_event_nodes.iter_mut() {
                handler.on_can_event(event, now);
            }
        }

        for node in nodes.iter_mut() {
            node.tick(now);
        }

        let mut outbox: heapless::Vec<(NodeAddress, heapless::Vec<u8, N>), Q> =
            heapless::Vec::new();
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
