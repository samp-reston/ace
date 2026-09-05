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

/// A failure to encode or decode a frame: the bytes did not match the wire format,
/// or the caller's buffer could not hold the encoded result.
///
/// This is deliberately **not** a transport error. A `CodecError` means retrying the
/// same bytes will fail identically, so a consumer should reject the frame rather
/// than retry the link.
///
/// There is deliberately no `TransportError` peer in this crate. Transport failure is
/// already modelled twice, in the places that own it: `FrameTransport::Error` and
/// `MessageTransport::Error` let each implementation choose its own type (a hosted
/// TCP transport wants `std::io::Error`, a CAN driver wants its own), and `LinkEvent`
/// carries connection lifecycle failures such as `ConnectionRefused` and
/// `ConnectionReset`. A concrete transport error here would be a third, competing
/// representation. The earlier `Driver` and `AddressNotReachable` variants of this
/// enum were never constructed for exactly that reason.
#[derive(Debug)]
pub enum CodecError {
    /// Bytes did not match the expected wire format.
    InvalidFrame(heapless::String<64>),

    /// A length field disagreed with the data actually present.
    LengthMismatch { expected: usize, actual: usize },

    /// The caller-provided output buffer was too small for the encoded frame.
    ///
    /// Distinct from `storage::CapacityError`, which reports one of the library's own
    /// bounded collections filling up. This one is about a buffer the caller supplied.
    BufferTooSmall,
}

/// Builds a bounded error message, truncating on a character boundary if needed.
///
/// Truncation is explicit here because `heapless::String::push_str` is
/// all-or-nothing: pushing an over-long message would leave the string **empty**,
/// discarding the diagnostic entirely rather than shortening it.
pub fn codec_err_str(s: &str) -> heapless::String<64> {
    let mut hs = heapless::String::new();

    // Longest prefix that both fits and ends on a character boundary.
    let mut end = s.len().min(64);
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }

    // Cannot fail: the prefix is at most 64 bytes by construction.
    let _ = hs.push_str(&s[..end]);
    hs
}

// endregion: Errors
