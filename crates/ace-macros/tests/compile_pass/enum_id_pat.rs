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
