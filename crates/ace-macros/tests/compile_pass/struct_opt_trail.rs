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

// Representative of ControlDTCSetting-style structs with optional trailing data
#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub struct WithOptional<'a> {
    pub setting_type: u8,
    #[frame(read_all)]
    pub option_record: &'a [u8],
}

fn main() {}
