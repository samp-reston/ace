use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub enum MissingId {
    Foo,
    Bar,
}

fn main() {}
