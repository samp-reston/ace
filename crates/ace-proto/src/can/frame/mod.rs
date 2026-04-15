pub mod classic;
pub mod fd;

pub use classic::{CanFrame, CanFrameMut};
pub use fd::{CanFdFrame, CanFdFrameMut};
