use ace_macros::FrameRead;

#[derive(FrameRead)]
pub enum MissingError {
    #[frame(id = "0x01")]
    Foo,
}

fn main() {}
