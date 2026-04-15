#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod can;
pub mod common;
pub mod doip;
pub mod error;
pub mod uds;

#[cfg(feature = "alloc")]
pub mod any_address;

#[cfg(feature = "alloc")]
pub use any_address::AnyAddress;

pub use can::{
    CanAddress, CanFdFrame, CanFdFrameMut, CanFrame, CanFrameMut, CanId, ExtendedCanId,
    StandardCanId,
};

pub use error::Error;

pub use doip::{DoipAddress, DoipFrame, DoipFrameMut, LogicalAddress};

pub use uds::{UdsFrame, UdsFrameMut};
