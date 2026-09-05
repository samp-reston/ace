//! Simulation harness over [`DiagnosticNode`].
//!
//! The node trait itself is a foundation abstraction and lives in `ace-core`.
//! What remains here is harness behaviour: [`SimNodeErased`] discards node errors
//! so a simulation run continues past a fault, which is deliberately *not* a
//! library contract, and [`SimRunner`] schedules nodes over a [`SimBus`].

// region: Imports

use crate::bus::SimBus;
use crate::clock::{Duration, Instant};
use crate::io::{InboundFrame, NodeAddress};
use ace_core::Vec;

pub use ace_core::node::DiagnosticNode;

// endregion: Imports

// region: SimNodeErased Trait

/// Object-safe view of a [`DiagnosticNode`] with its error type erased.
///
/// Exists so the runner can hold a heterogeneous slice of nodes whose `Error`
/// types differ. Errors are swallowed to keep a simulation running - never use
/// this on a path where a node error matters.
pub trait SimNodeErased<const MAX_FRAME: usize, const MAX_OUTBOX: usize> {
    fn address(&self) -> &NodeAddress;
    fn handle(&mut self, frame: &InboundFrame<'_>, now: Instant);
    fn tick(&mut self, now: Instant);
    fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
    ) -> usize;
}

/// Blanket impl - any DiagnosticNode becomes a SimNodeErased by discarding errors.
impl<const MAX_FRAME: usize, const MAX_OUTBOX: usize, T> SimNodeErased<MAX_FRAME, MAX_OUTBOX> for T
where
    T: DiagnosticNode<MAX_FRAME, MAX_OUTBOX>,
    T::Error: core::fmt::Debug,
{
    fn address(&self) -> &NodeAddress {
        DiagnosticNode::address(self)
    }

    fn handle(&mut self, frame: &InboundFrame<'_>, now: Instant) {
        if let Err(e) = DiagnosticNode::handle(self, frame, now) {
            // In no_std we cannot print but the error is available for
            // inspection via a debugger or a custom hook. Errors are
            // intentionally swallowed here - the simulation continues.
            let _ = e;
        }
    }

    fn tick(&mut self, now: Instant) {
        if let Err(e) = DiagnosticNode::tick(self, now) {
            let _ = e;
        }
    }

    fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
    ) -> usize {
        // Unlike `handle` and `tick`, this failure is not a node protocol error to be
        // swallowed - it means the harness gave the node too small a buffer. Losing
        // outbound messages would change what a deterministic run observes, so fail
        // loudly instead of continuing with a silently shortened run.
        DiagnosticNode::drain_outbox(self, out)
            .expect("outbox buffer smaller than the node's outbox")
    }
}

// endregion: SimNodeErased Trait

// region: SimRunner

/// Drives a collection of [`DiagnosticNode`]s connected via a [`SimBus`].
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight on the bus
pub struct SimRunner<const N: usize, const Q: usize> {
    bus: SimBus<N, Q>,
}

impl<const N: usize, const Q: usize> SimRunner<N, Q> {
    pub fn new(bus: SimBus<N, Q>) -> Self {
        Self { bus }
    }

    /// Ticks the simulation by `duration` microseconds, routing all delivered messages to their
    /// destination nodes and collecting their responses back onto the bus.
    ///
    /// Returns the number of messages delivered in this tick.
    pub fn tick(
        &mut self,
        nodes: &mut [&mut dyn SimNodeErased<N, Q>],
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
                let _ = node.handle(&frame, now);
            }
        }

        for node in nodes.iter_mut() {
            let _ = node.tick(now);
        }

        let mut outbox = Vec::new();

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

    pub fn bus(&mut self) -> &mut SimBus<N, Q> {
        &mut self.bus
    }
}

// endregion: SimRunner
