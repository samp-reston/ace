//! TcpSimRunner - drives SimNodeErased slices over a TcpSimBus.
//!
//! Extends the SimRunner concept for TCP-aware scenarios. Alongside message delivery it:
//!     - Delivers TcpEvents to nodes that implement TcpEventHandler
//!     - Handles connection state transitions visible to nodes
//!     - Drains node outboxes back onto the TCP bus (rejected if disconnected)

// region: Imports

use crate::{
    clock::{Duration, Instant},
    io::NodeAddress,
    node::SimNodeErased,
    tcp_bus::{TcpEvent, TcpSimBus},
};

// endregion: Imports

// region: TcpEventHandler

/// Optional trait for nodes that need to observe TCP connection events.
///
/// Nodes on TCP bus may implement this to react to connection establishent, reset, and closure.
/// The `TcpSimRunner` calls this after deliverying messages on each tick.
///
/// Not all nodes need TCP event awareness - only the `DoipTester` and gateway face nodes need it.
/// Nodes that do not implement this trait simply ignore connection events.
pub trait TcpEventHandler {
    fn on_tcp_event(&mut self, event: &TcpEvent, now: Instant);
}

// endregion: TcpEventHandler

// region: TcpSimRunner

/// Drives `SimNodeErased` slices over a `TcpSimBus`.
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight on the bus
pub struct TcpSimRunner<const N: usize, const Q: usize> {
    bus: TcpSimBus<N, Q>,
}

impl<const N: usize, const Q: usize> TcpSimRunner<N, Q> {
    pub fn new(bus: TcpSimBus<N, Q>) -> Self {
        Self { bus }
    }

    pub fn bus(&mut self) -> &mut TcpSimBus<N, Q> {
        &mut self.bus
    }

    pub fn now(&self) -> Instant {
        self.bus.now()
    }

    /// Ticks the simulation by `duration`.
    ///
    /// Order of operations per tick:
    ///     1. Advance bus clock, apply TCP fault injection, deliver due messages
    ///     2. Deliver TCP connection events to nodes implementing `TcpEventHandler`
    ///     3. Tick all nodes for time-based transitions
    ///     4. Drain node outboxes and enqueue onto the bus
    ///
    /// Returns the number of messages delivered.
    pub fn tick(
        &mut self,
        nodes: &mut [&mut dyn SimNodeErased<N, Q>],
        tcp_event_nodes: &mut [&mut dyn TcpEventHandler],
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

        let tcp_events: heapless::Vec<TcpEvent, 16> = self.bus.drain_events().collect();
        for event in &tcp_events {
            for handler in tcp_event_nodes.iter_mut() {
                handler.on_tcp_event(event, now);
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

// endregion: TcpSimRunner
