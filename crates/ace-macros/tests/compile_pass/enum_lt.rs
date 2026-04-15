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
pub struct BorrowedInner<'a> {
    pub tag: u8,
    #[frame(read_all)]
    pub data: &'a [u8],
}

// Enum that carries a lifetime because its newtype variant wraps a borrowed type
#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub enum EnumWithLifetime<'a> {
    #[frame(id = "0x01")]
    A(BorrowedInner<'a>),
    #[frame(id = "0x02")]
    B(BorrowedInner<'a>),
}

fn main() {}
