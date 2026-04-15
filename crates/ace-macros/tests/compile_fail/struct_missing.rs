use ace_macros::FrameRead;

#[derive(FrameRead)]
pub struct MissingError {
    pub value: u8,
}

fn main() {}
