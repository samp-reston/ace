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
