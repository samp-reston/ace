use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub struct ReadAllAndLength<'a> {
    pub len: u8,
    #[frame(read_all, length = "len as usize")]
    pub data: &'a [u8],
}

fn main() {}
