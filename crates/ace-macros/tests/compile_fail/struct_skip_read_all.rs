use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub struct SkipAndReadAll<'a> {
    #[frame(skip, read_all)]
    pub data: &'a [u8],
}

fn main() {}
