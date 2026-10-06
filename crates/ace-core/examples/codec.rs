use ace_core::{FrameRead, codec_err_str};

fn main() {
    let mut bytes: &[u8] = &[0x12, 0x34];
    let value = u16::decode(&mut bytes).expect("two-byte integer");

    assert_eq!(value, 0x1234);
    assert!(bytes.is_empty());
    assert_eq!(codec_err_str("invalid diagnostic frame").as_str(), "invalid diagnostic frame");
}
