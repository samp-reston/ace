extern crate ace_macros;

use ace_core::CodecError;
use ace_macros::FrameCodec;

#[derive(Debug)]
pub struct TestError;
impl From<TestError> for CodecError {
    fn from(_: TestError) -> Self {
        // was: _: CodecError
        CodecError::InvalidFrame(heapless::String::new())
    }
}

impl From<CodecError> for TestError {
    fn from(_: CodecError) -> Self {
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
