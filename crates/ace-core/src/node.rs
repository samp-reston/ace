//! The [`DiagnosticNode`] abstraction.
//!
//! A foundation trait, not a simulation type. This is the interface an external
//! scheduler uses to drive one diagnostic peer, and a bare-metal main loop is
//! exactly that scheduler: poll the CAN controller, [`handle`] the frame,
//! [`tick`] the timers, [`drain_outbox`] and write back. `ace-sim` drives the
//! same interface from a virtual bus.
//!
//! [`handle`]: DiagnosticNode::handle
//! [`tick`]: DiagnosticNode::tick
//! [`drain_outbox`]: DiagnosticNode::drain_outbox

// region: Imports

use crate::Vec;
use crate::clock::Instant;
use crate::io::NodeAddress;

// endregion: Imports

// region: DiagnosticNode Trait

/// One diagnostic peer - a server (ECU) or a client (tester).
///
/// Each node is a pure state machine. It receives messages via [`handle`],
/// produces outbound messages via [`drain_outbox`], and is advanced through time
/// by [`tick`]. The node owns no clock and no transport; the caller supplies both.
///
/// `MAX_FRAME` - max message payload bytes
/// `MAX_OUTBOX` - max messages held in the outbox simultaneously
///
/// [`handle`]: DiagnosticNode::handle
/// [`tick`]: DiagnosticNode::tick
/// [`drain_outbox`]: DiagnosticNode::drain_outbox
pub trait DiagnosticNode<const MAX_FRAME: usize, const MAX_OUTBOX: usize> {
    type Error: core::fmt::Debug;

    /// Returns this node's address.
    fn address(&self) -> &NodeAddress;

    /// Delivers an inbound message to the node.
    fn handle(&mut self, src: &NodeAddress, data: &[u8], now: Instant) -> Result<(), Self::Error>;

    /// Advances the node's internal state to the given time.
    ///
    /// Used to trigger timeouts and retries. Callers should invoke this on every
    /// scheduling cycle even if no messages were delivered.
    fn tick(&mut self, now: Instant) -> Result<(), Self::Error>;

    /// Drains all pending outbound messages from the node's outbox.
    ///
    /// Returns the number moved into `out` as `(dst, data)` pairs. Callers should
    /// invoke this after every `handle` and `tick` to collect output.
    ///
    /// All-or-nothing: if `out` cannot hold everything queued, nothing is moved and
    /// `Err` is returned, so the caller can retry with a larger buffer. Reporting a
    /// count while quietly dropping the excess would leave the caller believing
    /// messages were delivered that never left the node.
    fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
    ) -> Result<usize, crate::storage::CapacityError>;
}

// endregion: DiagnosticNode Trait
