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

// Representative of ReadMemoryByAddress-style structs where two length
// fields are derived from nibbles of a single format byte
#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub struct MultipleLength<'a> {
    pub addr_len: u8,
    pub size_len: u8,
    #[frame(length = "addr_len as usize")]
    pub address: &'a [u8],
    #[frame(length = "size_len as usize")]
    pub size: &'a [u8],
}

fn main() {}
