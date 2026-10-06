use ace_core::{FrameRead, FrameWrite};
use ace_doip::header::{DoipHeader, PayloadType, ProtocolVersion};

fn main() {
    let header = DoipHeader {
        protocol_version: ProtocolVersion::Iso13400_2012,
        inverse_protocol_version: 0xFD,
        payload_type: PayloadType::DiagnosticMessage,
        payload_length: 4,
    };

    let mut bytes = [0u8; 8];
    let mut output = &mut bytes[..];
    header.encode(&mut output).expect("header fits");
    assert!(output.is_empty());

    let mut input = &bytes[..];
    let decoded = DoipHeader::decode(&mut input).expect("valid header");
    assert_eq!(decoded, header);
    assert!(input.is_empty());
}
