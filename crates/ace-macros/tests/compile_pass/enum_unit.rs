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

// Representative of ServiceIdentifier - no lifetime, all unit variants
#[derive(Debug, Clone, Copy, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub enum UnitEnum {
    #[frame(id = "0x10")]
    Foo,
    #[frame(id = "0x20")]
    Bar,
    #[frame(id = "0x30")]
    Baz,
}

fn main() {}
