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
