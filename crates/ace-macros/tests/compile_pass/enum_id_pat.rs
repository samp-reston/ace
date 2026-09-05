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

// Representative of DTCSettingType, SessionType etc. with vendor-reserved ranges
#[derive(Debug, PartialEq, FrameCodec)]
#[frame(error = "TestError")]
pub enum WithCatchAll {
    #[frame(id = "0x01")]
    On,
    #[frame(id = "0x02")]
    Off,
    #[frame(id_pat = "0x03..=0xFF")]
    Reserved(u8),
}

fn main() {}
