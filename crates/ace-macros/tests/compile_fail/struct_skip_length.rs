use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub struct SkipAndLength<'a> {
    pub len: u8,
    #[frame(skip, length = "len as usize")]
    pub data: &'a [u8],
}

fn main() {}
