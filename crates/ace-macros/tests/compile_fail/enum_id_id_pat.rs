use ace_macros::FrameRead;

#[derive(FrameRead)]
#[frame(error = "TestError")]
pub enum IdAndIdPat {
    #[frame(id = "0x01", id_pat = "0x01..=0xFF")]
    Both(u8),
}

fn main() {}
