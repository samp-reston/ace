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

#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub struct BasicStruct {
    pub a: u8,
    pub b: u16,
    pub c: u32,
}

fn main() {}
