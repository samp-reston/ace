//! Drives a two-tier simulation: an outer connection-oriented bus and an inner
//! connectionless bus, bridged by nodes that sit on both.
//!
//! `SimRunner` handles the single-bus case. Nothing in `ace-sim` previously modelled
//! *two* buses composed together - the shape a DoIP gateway needs, bridging TCP (to the
//! tester) and CAN (to the ECUs). Consumers reimplemented that composition themselves,
//! and the order they chose - deliver, notify, tick, drain, immediately relay outbound
//! traffic onto the other bus before the next sub-tick - was asserted by convention
//! ("it seems right") rather than derived from anything. `TwoTierRunner` exists so that
//! order is written once, tested once, and is no longer available to get wrong per
//! integrator.
//!
//! Ordering per outer tick:
//!  1. Advance the outer bus, deliver due messages, drain its events.
//!  2. Notify every outer-tier node and every bridge of those events.
//!  3. Tick every outer-tier node and every bridge.
//!  4. Offer each delivered outer message to every outer-tier node and every bridge -
//!     each decides for itself whether the message is its own.
//!  5. Drain every outer-tier node's outbox onto the outer bus, and every bridge's
//!     *outer*-side outbox onto the outer bus - so a reply produced this step is visible
//!     to the very next inner sub-tick rather than waiting a full outer tick.
//!  6. Drain every bridge's *inner*-side outbox onto the inner bus, for the same reason.
//!  7. Run `inner_ticks_per_outer` inner sub-ticks, each repeating steps 1-6 for the
//!     inner bus and inner-tier nodes, with bridges relaying in both directions.
//!
//! A bridge's own outbox is drained on *both* buses every sub-tick specifically so a
//! response arriving on one side during a sub-tick reaches the other side before the
//! sub-tick loop ends, not after it - the two-drains-per-tick pattern the DoIP fixture
//! discovered was necessary and encoded as a topology-specific workaround.

// region: Imports

use crate::can_bus::{CanEvent, CanSimBus};
use crate::clock::{Duration, Instant};
use crate::io::{InboundFrame, NodeAddress};
use crate::node::SimNodeErased;
use crate::tcp_bus::{LinkEvent, TcpSimBus};
use ace_core::Vec;

// endregion: Imports

// region: Bridge

/// A node that sits on both tiers of a `TwoTierRunner`, relaying between them.
///
/// Distinct from `SimNodeErased` because a bridge has two frame sizes and two outboxes
/// (one per bus) rather than one - `SimNodeErased` cannot express that, and forcing a
/// bridge onto it would require picking one bus arbitrarily.
///
/// `OUTER_FRAME`/`OUTER_OUTBOX` size the outer-tier side (e.g. TCP); `INNER_FRAME`/
/// `INNER_OUTBOX` size the inner-tier side (e.g. CAN).
pub trait Bridge<
    const OUTER_FRAME: usize,
    const OUTER_OUTBOX: usize,
    const INNER_FRAME: usize,
    const INNER_OUTBOX: usize,
>
{
    /// This bridge's address on the outer tier.
    fn outer_address(&self) -> &NodeAddress;

    /// Offers a message delivered on the outer bus. The bridge decides for itself
    /// whether the message is its own.
    fn handle_outer(&mut self, frame: &InboundFrame<'_>, now: Instant);

    /// Offers a frame delivered on the inner bus. The bridge decides for itself
    /// whether the frame is its own.
    fn handle_inner(&mut self, frame: &InboundFrame<'_>, now: Instant);

    /// Advances any time-based state (session timers, pending transfers) on both sides.
    fn tick(&mut self, now: Instant);

    /// Drains messages the bridge wants sent on the outer bus.
    fn drain_outer_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, OUTER_FRAME>), OUTER_OUTBOX>,
    ) -> usize;

    /// Drains frames the bridge wants sent on the inner bus.
    fn drain_inner_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, INNER_FRAME>), INNER_OUTBOX>,
    ) -> usize;

    /// Notifies the bridge of an outer-tier connection lifecycle event (e.g. a TCP
    /// connection reset). Nodes uninterested in connection state may no-op.
    fn on_outer_event(&mut self, event: &LinkEvent, now: Instant);

    /// Notifies the bridge of an inner-tier bus event (e.g. CAN bus-off). Nodes
    /// uninterested in bus state may no-op.
    fn on_inner_event(&mut self, event: &CanEvent, now: Instant);
}

// endregion: Bridge

// region: OuterEventSink

/// An outer-tier node that also wants outer-tier connection lifecycle events.
///
/// Kept separate from `SimNodeErased` rather than added to it: `SimNodeErased` is shared
/// with the single-bus `SimRunner`, where there is no connection concept to report, and
/// most outer-tier nodes (in the DoIP case, none of the ECUs - they are never outer-tier
/// at all) have nothing to do with a link event. A tester learning a connection has
/// activated is the one case that needs it, so it is opt-in rather than forcing every
/// `SimNodeErased` implementor to grow a no-op method.
pub trait OuterEventSink {
    fn on_outer_event(&mut self, event: &LinkEvent, now: Instant);
}

// endregion: OuterEventSink

// region: OuterNode

/// An outer-tier plain node that also wants outer-tier connection events.
///
/// Exists so a single `&mut dyn` reference can be handed to the runner once, rather than
/// needing two live mutable borrows of the same node (one per trait) at once - which
/// `SimNodeErased` and `OuterEventSink` alone would require of a caller with exactly one
/// node value in hand.
pub trait OuterNode<const OUTER_FRAME: usize, const OUTER_OUTBOX: usize>:
    SimNodeErased<OUTER_FRAME, OUTER_OUTBOX> + OuterEventSink
{
}

impl<const OUTER_FRAME: usize, const OUTER_OUTBOX: usize, T> OuterNode<OUTER_FRAME, OUTER_OUTBOX>
    for T
where
    T: SimNodeErased<OUTER_FRAME, OUTER_OUTBOX> + OuterEventSink,
{
}

// endregion: OuterNode

// region: TwoTierRunner

/// Drives outer-tier nodes, inner-tier nodes, and bridges between them over a
/// `TcpSimBus` (outer) and `CanSimBus` (inner).
///
/// Named for the roles, not the protocols: nothing here parses DoIP or ISO-TP. `OUTER_*`
/// and `INNER_*` size the two buses and their event queues independently, since a
/// connection-oriented outer bus and a connectionless inner bus have unrelated capacity
/// needs.
pub struct TwoTierRunner<
    const OUTER_FRAME: usize,
    const OUTER_QUEUED: usize,
    const OUTER_EVENTS: usize,
    const OUTER_CONNECTIONS: usize,
    const OUTER_OUTBOX: usize,
    const INNER_FRAME: usize,
    const INNER_QUEUED: usize,
    const INNER_EVENTS: usize,
    const INNER_OUTBOX: usize,
> {
    outer_bus: TcpSimBus<OUTER_FRAME, OUTER_QUEUED, OUTER_EVENTS, OUTER_CONNECTIONS>,
    inner_bus: CanSimBus<INNER_FRAME, INNER_QUEUED>,
    inner_ticks_per_outer: usize,
}

impl<
    const OUTER_FRAME: usize,
    const OUTER_QUEUED: usize,
    const OUTER_EVENTS: usize,
    const OUTER_CONNECTIONS: usize,
    const OUTER_OUTBOX: usize,
    const INNER_FRAME: usize,
    const INNER_QUEUED: usize,
    const INNER_EVENTS: usize,
    const INNER_OUTBOX: usize,
>
    TwoTierRunner<
        OUTER_FRAME,
        OUTER_QUEUED,
        OUTER_EVENTS,
        OUTER_CONNECTIONS,
        OUTER_OUTBOX,
        INNER_FRAME,
        INNER_QUEUED,
        INNER_EVENTS,
        INNER_OUTBOX,
    >
{
    pub fn new(
        outer_bus: TcpSimBus<OUTER_FRAME, OUTER_QUEUED, OUTER_EVENTS, OUTER_CONNECTIONS>,
        inner_bus: CanSimBus<INNER_FRAME, INNER_QUEUED>,
        inner_ticks_per_outer: usize,
    ) -> Self {
        Self {
            outer_bus,
            inner_bus,
            inner_ticks_per_outer,
        }
    }

    pub fn outer_bus(
        &mut self,
    ) -> &mut TcpSimBus<OUTER_FRAME, OUTER_QUEUED, OUTER_EVENTS, OUTER_CONNECTIONS> {
        &mut self.outer_bus
    }

    pub fn inner_bus(&mut self) -> &mut CanSimBus<INNER_FRAME, INNER_QUEUED> {
        &mut self.inner_bus
    }

    pub fn now(&self) -> Instant {
        self.outer_bus.now()
    }

    /// Advances one full step: one outer tick, then `inner_ticks_per_outer` inner ticks.
    ///
    /// `outer_nodes`/`inner_nodes` are the plain, single-tier participants (e.g. the
    /// tester on the outer bus, ECUs on the inner bus). `bridges` sit on both.
    /// `outer_nodes` also receives outer-tier connection events through `OuterNode`,
    /// since a plain outer-tier node normally needs to know about them (the tester
    /// learning a connection has activated is the motivating case).
    pub fn tick(
        &mut self,
        outer_nodes: &mut [&mut dyn OuterNode<OUTER_FRAME, OUTER_OUTBOX>],
        inner_nodes: &mut [&mut dyn SimNodeErased<INNER_FRAME, INNER_OUTBOX>],
        bridges: &mut [&mut dyn Bridge<OUTER_FRAME, OUTER_OUTBOX, INNER_FRAME, INNER_OUTBOX>],
        outer_duration: Duration,
        inner_duration: Duration,
    ) {
        self.tick_outer(outer_nodes, bridges, outer_duration);

        for _ in 0..self.inner_ticks_per_outer {
            self.tick_inner(inner_nodes, bridges, inner_duration);
        }
    }

    /// One outer-tier sub-tick: deliver, notify, tick, offer, relay.
    fn tick_outer(
        &mut self,
        outer_nodes: &mut [&mut dyn OuterNode<OUTER_FRAME, OUTER_OUTBOX>],
        bridges: &mut [&mut dyn Bridge<OUTER_FRAME, OUTER_OUTBOX, INNER_FRAME, INNER_OUTBOX>],
        duration: Duration,
    ) {
        let delivered = self.outer_bus.tick(duration);
        let now = self.outer_bus.now();

        let events: Vec<LinkEvent, OUTER_EVENTS> =
            Vec::from_iter_checked(self.outer_bus.drain_events())
                .expect("event buffer shares OUTER_EVENTS with the bus queue");
        for event in &events {
            for node in outer_nodes.iter_mut() {
                node.on_outer_event(event, now);
            }
            for bridge in bridges.iter_mut() {
                bridge.on_outer_event(event, now);
            }
        }

        for node in outer_nodes.iter_mut() {
            node.tick(now);
        }
        for bridge in bridges.iter_mut() {
            bridge.tick(now);
        }

        for envelope in &delivered {
            let frame =
                InboundFrame::new(envelope.src.clone(), envelope.dst.clone(), &envelope.data);

            for node in outer_nodes.iter_mut() {
                node.handle(&frame, now);
            }
            for bridge in bridges.iter_mut() {
                bridge.handle_outer(&frame, now);
            }
        }

        let mut outbox: Vec<(NodeAddress, Vec<u8, OUTER_FRAME>), OUTER_OUTBOX> = Vec::new();
        for node in outer_nodes.iter_mut() {
            outbox.clear();
            node.drain_outbox(&mut outbox);

            let src = node.address().clone();
            for (dst, data) in outbox.iter() {
                self.outer_bus.send(src.clone(), dst.clone(), data);
            }
        }
        for bridge in bridges.iter_mut() {
            outbox.clear();
            bridge.drain_outer_outbox(&mut outbox);

            let src = bridge.outer_address().clone();
            for (dst, data) in outbox.iter() {
                self.outer_bus.send(src.clone(), dst.clone(), data);
            }
        }
    }

    /// One inner-tier sub-tick: deliver, notify, tick, offer, relay both ways.
    fn tick_inner(
        &mut self,
        inner_nodes: &mut [&mut dyn SimNodeErased<INNER_FRAME, INNER_OUTBOX>],
        bridges: &mut [&mut dyn Bridge<OUTER_FRAME, OUTER_OUTBOX, INNER_FRAME, INNER_OUTBOX>],
        duration: Duration,
    ) {
        let delivered = self.inner_bus.tick(duration);
        let now = self.inner_bus.now();

        let events: Vec<CanEvent, INNER_EVENTS> =
            Vec::from_iter_checked(self.inner_bus.drain_events())
                .expect("event buffer shares INNER_EVENTS with the bus queue");
        for event in &events {
            for bridge in bridges.iter_mut() {
                bridge.on_inner_event(event, now);
            }
        }

        for node in inner_nodes.iter_mut() {
            node.tick(now);
        }
        for bridge in bridges.iter_mut() {
            bridge.tick(now);
        }

        for envelope in &delivered {
            let frame =
                InboundFrame::new(envelope.src.clone(), envelope.dst.clone(), &envelope.data);

            for node in inner_nodes.iter_mut() {
                node.handle(&frame, now);
            }
            for bridge in bridges.iter_mut() {
                bridge.handle_inner(&frame, now);
            }
        }

        let mut inner_outbox: Vec<(NodeAddress, Vec<u8, INNER_FRAME>), INNER_OUTBOX> = Vec::new();
        for node in inner_nodes.iter_mut() {
            inner_outbox.clear();
            node.drain_outbox(&mut inner_outbox);

            let src = node.address().clone();
            for (dst, data) in inner_outbox.iter() {
                self.inner_bus.send(src.clone(), dst.clone(), data);
            }
        }
        for bridge in bridges.iter_mut() {
            inner_outbox.clear();
            bridge.drain_inner_outbox(&mut inner_outbox);

            let src = bridge.outer_address().clone();
            for (dst, data) in inner_outbox.iter() {
                self.inner_bus.send(src.clone(), dst.clone(), data);
            }
        }

        // A response the bridge produced from the frame just delivered must reach the
        // outer bus before this sub-tick ends, not after the whole loop of
        // `inner_ticks_per_outer` completes - otherwise a reply is delayed by however
        // many inner ticks remain, which is the two-drains-per-tick fix the DoIP fixture
        // needed and is now built in rather than rediscovered per integrator.
        let mut outer_outbox: Vec<(NodeAddress, Vec<u8, OUTER_FRAME>), OUTER_OUTBOX> = Vec::new();
        for bridge in bridges.iter_mut() {
            outer_outbox.clear();
            bridge.drain_outer_outbox(&mut outer_outbox);

            let src = bridge.outer_address().clone();
            for (dst, data) in outer_outbox.iter() {
                self.outer_bus.send(src.clone(), dst.clone(), data);
            }
        }
    }
}

// endregion: TwoTierRunner
