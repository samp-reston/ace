// region: Imports

use crate::clock::Instant;

// endregion: Imports

// region: Address

/// A logical node address in the simulation network.
///
/// Maps to a CAN ID, DoIP logical address, or any other addressing scheme depending on the
/// protocol layer in use.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAddress(pub u32);

// endregion: Address

// region: RawMessage

/// A raw byte message between two nodes.
///
/// Used by low-level runtime implementers working diectly with frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMessage<const N: usize> {
    pub src: NodeAddress,
    pub dst: NodeAddress,
    pub data: heapless::Vec<u8, N>,
    pub timestamp: Instant,
}

// endregion: RawMessage

// region: FrameTransport Trait

/// Low-level transport trait for frame-oriented communication.
///
/// Implementers of CAN/DoIP runtimes use this trait. The simulation replaces this with an
/// in-memory channel that can inject faults.
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
