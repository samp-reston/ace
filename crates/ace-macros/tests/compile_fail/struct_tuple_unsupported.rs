use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub struct TupleStruct(u8, u16);

fn main() {}
