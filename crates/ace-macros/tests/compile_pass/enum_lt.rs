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
