//! Addressing, raw messages, transport traits, and connection lifecycle events.
//!
//! All foundation abstractions. A production integrator implements
//! [`FrameTransport`] over a real CAN controller or socket and consumes
//! [`LinkEvent`] from a real connection; `ace-sim` implements the same
//! traits over an in-memory bus that can inject faults.

// region: Imports

use crate::Vec;
use crate::clock::Instant;

// endregion: Imports

// region: Address

/// A logical node address.
///
/// Maps to a CAN ID, DoIP logical address, or any other addressing scheme depending on the
/// protocol layer in use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAddress(pub u32);

// endregion: Address

// region: RawMessage

/// A raw byte message between two nodes.
///
/// Used by low-level runtime implementers working directly with frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMessage<const MAX_DATA: usize> {
    pub src: NodeAddress,
    pub dst: NodeAddress,
    pub data: Vec<u8, MAX_DATA>,
    pub timestamp: Instant,
}

// endregion: RawMessage

// region: LinkEvent

/// A lifecycle transition on a connection-oriented link.
///
/// Named neutrally rather than after a transport: these are the transitions a
/// DoIP TCP socket reports, and a connection-oriented link on another transport
/// reports the same set. A simulated bus produces them from its fault injector; a
/// real socket produces them from the network. Consumers must not distinguish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEvent {
    /// A connection request from `from` to `to` was accepted.
    ConnectionEstablished { from: NodeAddress, to: NodeAddress },

    /// A connection request was refused.
    ConnectionRefused { from: NodeAddress, to: NodeAddress },

    /// A connection request timed out.
    ConnectionTimeout { from: NodeAddress, to: NodeAddress },

    /// An established connection was reset. Both nodes should treat this as a
    /// TCP RST or the transport's equivalent.
    ConnectionReset { from: NodeAddress, to: NodeAddress },

    /// The connection was closed cleanly by one of the nodes.
    ConnectionClosed { from: NodeAddress, to: NodeAddress },
}

// endregion: LinkEvent

// region: FrameTransport Trait

/// Low-level transport trait for frame-oriented communication.
///
/// Implementers of CAN/DoIP runtimes use this trait. A simulation replaces the
/// implementation with an in-memory channel that can inject faults.
pub trait FrameTransport<const N: usize> {
    type Error: core::fmt::Debug;

    /// Sends a raw frame to the given destination.
    fn send(&mut self, dst: &NodeAddress, data: &[u8]) -> Result<(), Self::Error>;

    /// Receives the next available raw frame, if any. Returns `None` if no frame is available
    fn recv(&mut self) -> Option<RawMessage<N>>;
}

// endregion: FrameTransport Trait

// region: MessageTransport Trait

/// High-level transport trait for named message communication.
///
/// Application developers building on top of UDS/DoIP use this trait. Messages are typed - the
/// transport handles serialisation internally.
pub trait MessageTransport {
    type Message: core::fmt::Debug;
    type Error: core::fmt::Debug;

    /// Sends a typed message to the given destination
    fn send(&mut self, dst: &NodeAddress, message: Self::Message) -> Result<(), Self::Error>;

    /// Receives the next available typed message, if any.
    fn recv(&mut self) -> Option<(NodeAddress, Self::Message)>;
}

// endregion: MessageTransport Trait
