use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub union NotSupported {
    pub a: u8,
    pub b: u16,
}

fn main() {}
