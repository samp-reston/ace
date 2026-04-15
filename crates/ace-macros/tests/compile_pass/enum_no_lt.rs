extern crate ace_macros;

use ace_core::DiagError;
use ace_macros::FrameCodec;

#[derive(Debug)]
pub struct TestError;
impl From<TestError> for DiagError {
    fn from(_: TestError) -> Self {
        // was: _: DiagError
        DiagError::InvalidFrame(heapless::String::new())
    }
}

impl From<DiagError> for TestError {
    fn from(_: DiagError) -> Self {
        TestError
    }
}

// Regression: with_lifetime_a was injecting 'a into enums that don't
// declare one, causing E0107 on unit enums like ServiceIdentifier
#[derive(Debug, Clone, Copy, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub enum NoLifetimeEnum {
    #[frame(id = "0x01")]
    A,
    #[frame(id = "0x02")]
    B,
    #[frame(id = "0x03")]
    C,
}

fn main() {}
