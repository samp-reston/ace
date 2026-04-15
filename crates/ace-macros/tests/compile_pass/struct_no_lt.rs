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

// Regression: with_lifetime_a was injecting 'a into structs that don't
// declare one, causing E0107 "struct takes 0 lifetime arguments but 1 supplied"
#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub struct NoLifetime {
    pub x: u8,
    pub y: u16,
    pub z: [u8; 4],
}

fn main() {}
