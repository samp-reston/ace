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
pub struct Inner {
    pub value: u8,
}

#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub enum NewtypeEnum {
    #[frame(id = "0x01")]
    A(Inner),
    #[frame(id = "0x02")]
    B(Inner),
}

fn main() {}
