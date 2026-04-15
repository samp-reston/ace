#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod codec;
pub use codec::{take_n, FrameCodec, FrameRead, FrameWrite, Writer};

pub mod iter;
pub use iter::FrameIter;

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
