//! TcpSimRunner - drives SimNodeErased slices over a TcpSimBus.
//!
//! Extends the SimRunner concept for TCP-aware scenarios. Alongside message delivery it:
//!     - Delivers TcpEvents to nodes that implement TcpEventHandler
//!     - Handles connection state transitions visible to nodes
//!     - Drains node outboxes back onto the TCP bus (rejected if disconnected)

// region: Imports

use crate::{
    clock::{Duration, Instant},
    io::{InboundFrame, NodeAddress},
    node::SimNodeErased,
    tcp_bus::{LinkEvent, TcpSimBus},
};
use ace_core::Vec;

// endregion: Imports

// region: TcpEventHandler

/// Optional trait for nodes that need to observe TCP connection events.
///
/// Nodes on TCP bus may implement this to react to connection establishment, reset, and closure.
/// The `TcpSimRunner` calls this after delivering messages on each tick.
///
/// Not all nodes need TCP event awareness - only the `DoipTester` and gateway face nodes need it.
/// Nodes that do not implement this trait simply ignore connection events.
pub trait TcpEventHandler {
    fn on_tcp_event(&mut self, event: &LinkEvent, now: Instant);
}

// endregion: TcpEventHandler

// region: TcpSimRunner

/// Drives `SimNodeErased` slices over a `TcpSimBus`.
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight on the bus
pub struct TcpSimRunner<
    const MAX_DATA: usize,
    const MAX_QUEUED: usize,
    const TCP_MAX_EVENTS: usize,
    const MAX_OUTBOX: usize,
> {
    bus: TcpSimBus<MAX_DATA, MAX_QUEUED, TCP_MAX_EVENTS>,
}

impl<
    const MAX_DATA: usize,
    const MAX_QUEUED: usize,
    const TCP_MAX_EVENTS: usize,
    const MAX_OUTBOX: usize,
> TcpSimRunner<MAX_DATA, MAX_QUEUED, TCP_MAX_EVENTS, MAX_OUTBOX>
{
    pub fn new(bus: TcpSimBus<MAX_DATA, MAX_QUEUED, TCP_MAX_EVENTS>) -> Self {
        Self { bus }
    }

    pub fn bus(&mut self) -> &mut TcpSimBus<MAX_DATA, MAX_QUEUED, TCP_MAX_EVENTS> {
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
        nodes: &mut [&mut dyn SimNodeErased<MAX_DATA, MAX_OUTBOX>],
        tcp_event_nodes: &mut [&mut dyn TcpEventHandler],
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

        // Shares TCP_MAX_EVENTS with the bus queue, so this cannot overflow.
        let tcp_events: Vec<LinkEvent, TCP_MAX_EVENTS> =
            Vec::from_iter_checked(self.bus.drain_events())
                .expect("event buffer shares TCP_MAX_EVENTS with the bus queue");
        for event in &tcp_events {
            for handler in tcp_event_nodes.iter_mut() {
                handler.on_tcp_event(event, now);
            }
        }

        for node in nodes.iter_mut() {
            node.tick(now);
        }

        let mut outbox: Vec<(NodeAddress, Vec<u8, MAX_DATA>), MAX_OUTBOX> = Vec::new();
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
