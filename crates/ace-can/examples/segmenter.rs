use ace_can::{IsoTpAddressingMode, SegmentResult, Segmenter, SegmenterConfig};

fn main() {
    let mut segmenter = Segmenter::<32>::new(SegmenterConfig::classic(IsoTpAddressingMode::Normal));
    segmenter.start(&[0x22, 0xF1, 0x90]).expect("payload fits");

    let mut frame = [0u8; 8];
    match segmenter.next_frame(&mut frame).expect("frame generation") {
        SegmentResult::Frame { len } => assert_eq!(&frame[..len], &[0x03, 0x22, 0xF1, 0x90]),
        other => panic!("unexpected ISO-TP result: {other:?}"),
    }

    assert!(matches!(
        segmenter.next_frame(&mut frame).expect("completion"),
        SegmentResult::Complete
    ));
}
