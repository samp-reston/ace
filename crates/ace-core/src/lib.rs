#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod codec;
pub use codec::{FrameCodec, FrameRead, FrameWrite, Writer, take_n};

pub mod iter;
pub use iter::FrameIter;

pub mod storage;
pub use storage::Vec;

pub mod clock;
pub use clock::{Clock, Duration, Instant};

pub mod io;
pub use io::{FrameTransport, LinkEvent, MessageTransport, NodeAddress, RawMessage};

pub mod node;
pub use node::DiagnosticNode;

pub mod rng;
pub use rng::Rng;

// region: Addressing

pub trait DiagnosticAddress: Clone + core::fmt::Debug {
    fn address_mode(&self) -> AddressMode;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressMode {
    Physical,
    Functional,
}

// endregion: Addressing

// region: Errors

#[derive(Debug)]
pub enum DiagError {
    Timeout,
    InvalidFrame(heapless::String<64>),
    BufferOverflow,
    Driver(heapless::String<64>),
    AddressNotReachable,
    LengthMismatch { expected: usize, actual: usize },
}

pub fn diag_err_str(s: &str) -> heapless::String<64> {
    let mut hs = heapless::String::new();
    let _ = hs.push_str(s);
    hs
}

// endregion: Errors
