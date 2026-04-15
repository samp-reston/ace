// region: Imports

use crate::bus::SimBus;
use crate::clock::{Duration, Instant};
use crate::io::NodeAddress;
use heapless::Vec;

// endregion: Imports

// region: SimNode Trait

/// A participant in the simulation - either a sever (ECU) or client (tester).
///
/// Each node is a pure state machine. It receives messages via `handle`, produces outbound
/// messages via `drain_outbox`, and can be ticket for time-based state transitions.
///
/// `N` - max message payload bytes
/// `Q` - max messages in outbox simultaneously
pub trait SimNode<const N: usize, const Q: usize> {
    type Error: core::fmt::Debug;

    /// Returns this node's address on the simulation bus.
    fn address(&self) -> &NodeAddress;

    /// Delivers an inbound message to the node.
    fn handle(&mut self, src: &NodeAddress, data: &[u8], now: Instant) -> Result<(), Self::Error>;

    /// Advances the node's internal state to the given time.
    ///
    /// Used to trigger timeouts and retries. Called by the simulation bus on every tick even if no
    /// messages were delivered.
    fn tick(&mut self, now: Instant) -> Result<(), Self::Error>;

    /// Drains all pending outbound messages from the node's outbox.
    ///
    /// Return an iterator of `(dst, data)` pairs. The bus calls this after every `handle` and
    /// `tick` call to collect and route output.
    fn drain_outbox(
        &mut self,
        out: &mut heapless::Vec<(NodeAddress, heapless::Vec<u8, N>), Q>,
    ) -> usize;
}

// endregion: SimNode Trait

// region: SimNodeErased Trait

pub trait SimNodeErased<const N: usize, const Q: usize> {
    fn address(&self) -> &NodeAddress;
    fn handle(&mut self, src: &NodeAddress, data: &[u8], now: Instant);
    fn tick(&mut self, now: Instant);
    fn drain_outbox(&mut self, out: &mut Vec<(NodeAddress, Vec<u8, N>), Q>) -> usize;
}

/// Blanket impl - any SimNode becomes a SimNodeErased by discarding errors.
impl<const N: usize, const Q: usize, T> SimNodeErased<N, Q> for T
where
    T: SimNode<N, Q>,
    T::Error: core::fmt::Debug,
{
    fn address(&self) -> &NodeAddress {
        SimNode::address(self)
    }

    fn handle(&mut self, src: &NodeAddress, data: &[u8], now: Instant) {
        if let Err(e) = SimNode::handle(self, src, data, now) {
            // In no_std we cannot print but the error is available for
            // inspection via a debugger or a custom hook. Errors are
            // intentionally swallowed here - the simulation continues.
            let _ = e;
        }
    }

    fn tick(&mut self, now: Instant) {
        if let Err(e) = SimNode::tick(self, now) {
            let _ = e;
        }
    }

    fn drain_outbox(&mut self, out: &mut Vec<(NodeAddress, Vec<u8, N>), Q>) -> usize {
        SimNode::drain_outbox(self, out)
    }
}

// endregion: SimNodeErased Trait

// region: SimRunner

/// Drives a collection of [`SimNode`]s connected via a [`SimBus`].
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight on the bus
/// `S` - max nodes in the simulation
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
            for node in nodes.iter_mut() {
                if *node.address() == envelope.dst {
                    let _ = node.handle(&envelope.src, &envelope.data, now);
                    count += 1;
                }
            }
        }

        for node in nodes.iter_mut() {
            let _ = node.tick(now);
        }

        let mut outbox = heapless::Vec::new();

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
