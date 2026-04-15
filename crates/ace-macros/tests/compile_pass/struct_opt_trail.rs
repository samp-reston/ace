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

// Representative of ControlDTCSetting-style structs with optional trailing data
#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub struct WithOptional<'a> {
    pub setting_type: u8,
    #[frame(read_all)]
    pub option_record: &'a [u8],
}

fn main() {}
