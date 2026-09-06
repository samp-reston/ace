extern crate std;
use std::vec::Vec;

use ace_can::isotp::address::IsoTpAddressingMode;
use ace_can::isotp::reassembler::{ReassembleResult, Reassembler, ReassemblerConfig};
use ace_can::isotp::segmenter::{SegmentResult, Segmenter, SegmenterConfig};

// region: Helpers

/// Pipes a payload through the segmenter and reassembler end-to-end.
///
/// The segmenter produces raw PCI bytes. For extended/mixed addressing
/// the address byte is stripped from received frames before feeding the
/// reassembler, and prepended to FC frames before passing back to the
/// segmenter - mirroring what a real transport boundary layer would do.
fn roundtrip<const N: usize>(
    payload: &[u8],
    seg_config: SegmenterConfig,
    rsm_config: ReassemblerConfig,
) -> Vec<u8> {
    let mut segmenter: Segmenter<N> = Segmenter::new(seg_config);
    let mut reassembler: Reassembler<N> = Reassembler::new(rsm_config);

    segmenter.start(payload).expect("start should not fail");

    let mut out_buf = [0u8; 70];
    let mut reassembled_len = 0;

    loop {
        match segmenter
            .next_frame(&mut out_buf)
            .expect("next_frame should not fail")
        {
            SegmentResult::Complete => break,
            SegmentResult::WaitForFlowControl => {
                panic!("unexpected WaitForFlowControl with block_size=0");
            }
            SegmentResult::Frame { len } => {
                // Segmenter produces raw PCI bytes - feed directly, no stripping
                match reassembler
                    .feed(&out_buf[..len])
                    .expect("feed should not fail")
                {
                    ReassembleResult::Complete { len } => {
                        reassembled_len = len;
                        break;
                    }
                    ReassembleResult::InProgress => {}
                    ReassembleResult::FlowControl { frame, len: fc_len } => {
                        segmenter
                            .handle_flow_control(&frame[..fc_len])
                            .expect("handle_flow_control should not fail");
                    }
                    ReassembleResult::SessionAborted { .. } => {
                        panic!("unexpected session abort");
                    }
                }
            }
        }
    }

    reassembler
        .message(reassembled_len)
        .expect("message should be available after Complete")
        .to_vec()
}
// endregion: Helpers

// region: Normal addressing - classic CAN

#[test]
fn rt_single_frame_normal_classic() {
    let payload = b"hello";
    let result = roundtrip::<64>(
        payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Normal),
        ReassemblerConfig::new(IsoTpAddressingMode::Normal),
    );
    assert_eq!(result, payload);
}

#[test]
fn rt_multi_frame_normal_classic() {
    let payload = b"hello world this is a multi frame message";
    let result = roundtrip::<64>(
        payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Normal),
        ReassemblerConfig::new(IsoTpAddressingMode::Normal),
    );
    assert_eq!(result, payload);
}

#[test]
fn rt_max_classic_payload_normal() {
    let payload = [0xABu8; 4095];
    let result = roundtrip::<4095>(
        &payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Normal),
        ReassemblerConfig::new(IsoTpAddressingMode::Normal),
    );
    assert_eq!(result, payload);
}

// endregion: Normal addressing - classic CAN

// region: Extended addressing - classic CAN

#[test]
fn rt_single_frame_extended_classic() {
    let payload = b"hi";
    let result = roundtrip::<64>(
        payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Extended),
        ReassemblerConfig::new(IsoTpAddressingMode::Extended),
    );
    assert_eq!(result, payload);
}

#[test]
fn rt_multi_frame_extended_classic() {
    let payload = b"extended addressing multi frame test payload";
    let result = roundtrip::<64>(
        payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Extended),
        ReassemblerConfig::new(IsoTpAddressingMode::Extended),
    );
    assert_eq!(result, payload);
}

// endregion: Extended addressing - classic CAN

// region: Mixed addressing - classic CAN

#[test]
fn rt_single_frame_mixed_classic() {
    let payload = b"mix";
    let result = roundtrip::<64>(
        payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Mixed),
        ReassemblerConfig::new(IsoTpAddressingMode::Mixed),
    );
    assert_eq!(result, payload);
}

#[test]
fn rt_multi_frame_mixed_classic() {
    let payload = b"mixed addressing multi frame test payload here";
    let result = roundtrip::<64>(
        payload,
        SegmenterConfig::classic(IsoTpAddressingMode::Mixed),
        ReassemblerConfig::new(IsoTpAddressingMode::Mixed),
    );
    assert_eq!(result, payload);
}

// endregion: Mixed addressing - classic CAN

// region: Normal addressing - CAN FD

#[test]
fn rt_single_frame_normal_fd() {
    let payload = b"this fits in a single CAN FD frame easily";
    let result = roundtrip::<128>(
        payload,
        SegmenterConfig::fd(IsoTpAddressingMode::Normal),
        ReassemblerConfig::new(IsoTpAddressingMode::Normal),
    );
    assert_eq!(result, payload);
}

#[test]
fn rt_multi_frame_normal_fd() {
    let payload = [0xCDu8; 256];
    let result = roundtrip::<256>(
        &payload,
        SegmenterConfig::fd(IsoTpAddressingMode::Normal),
        ReassemblerConfig::new(IsoTpAddressingMode::Normal),
    );
    assert_eq!(result, &payload);
}

#[test]
fn rt_fd_escape_sequence() {
    let payload = [0xEFu8; 5000];
    let result = roundtrip::<5000>(
        &payload,
        SegmenterConfig::fd(IsoTpAddressingMode::Normal),
        ReassemblerConfig::new(IsoTpAddressingMode::Normal),
    );
    assert_eq!(result, &payload);
}

// endregion: Normal addressing - CAN FD

// region: Flow control - block size

fn roundtrip_with_block_size<const N: usize>(
    payload: &[u8],
    block_size: u8,
    addressing_mode: IsoTpAddressingMode,
) -> Vec<u8> {
    let addr_offset = addressing_mode.pci_offset();
    let seg_config = SegmenterConfig::classic(addressing_mode.clone());
    let rsm_config = ReassemblerConfig {
        addressing_mode: addressing_mode.clone(),
        block_size,
        st_min: 0,
    };

    let mut segmenter: Segmenter<N> = Segmenter::new(seg_config);
    let mut reassembler: Reassembler<N> = Reassembler::new(rsm_config);

    segmenter.start(payload).expect("start should not fail");

    let mut out_buf = [0u8; 70];
    let mut reassembled_len = 0;

    loop {
        match segmenter
            .next_frame(&mut out_buf)
            .expect("next_frame should not fail")
        {
            SegmentResult::Complete => break,

            SegmentResult::WaitForFlowControl => {
                // Unreachable when the reassembler honours its own advertised block size:
                // it emits the next flow control on receiving the last frame of a block,
                // which is fed to the segmenter below *before* it is asked for another
                // frame. Reaching here means the sender is blocked on a flow control that
                // will never come - a permanently stalled transfer.
                //
                // This arm used to build the frame itself
                // (`let fc = [0x30u8, block_size, 0x00];`) and hand it to the segmenter,
                // which is why these tests passed while the reassembler emitted no flow
                // control at all.
                panic!(
                    "segmenter is waiting for flow control the reassembler never sent \
                     (block_size={block_size})"
                );
            }

            SegmentResult::Frame { len } => {
                let pci_bytes = &out_buf[addr_offset..len];
                match reassembler.feed(pci_bytes).expect("feed should not fail") {
                    ReassembleResult::Complete { len } => {
                        reassembled_len = len;
                        break;
                    }
                    ReassembleResult::InProgress => {}
                    ReassembleResult::FlowControl { frame, len: fc_len } => {
                        segmenter
                            .handle_flow_control(&frame[..fc_len])
                            .expect("handle_flow_control should not fail");
                    }
                    ReassembleResult::SessionAborted { .. } => {
                        panic!("unexpected session abort");
                    }
                }
            }
        }
    }

    reassembler
        .message(reassembled_len)
        .expect("message should be available")
        .to_vec()
}

#[test]
fn rt_block_size_1() {
    let payload = b"block size one means fc after every consecutive frame";
    let result = roundtrip_with_block_size::<64>(payload, 1, IsoTpAddressingMode::Normal);
    assert_eq!(result, payload);
}

#[test]
fn rt_block_size_3() {
    let payload = b"block size three means fc after every third consecutive frame";
    let result = roundtrip_with_block_size::<64>(payload, 3, IsoTpAddressingMode::Normal);
    assert_eq!(result, payload);
}

/// The block boundary itself: flow control is due after exactly `block_size` consecutive
/// frames, not before and not later.
///
/// Driven by hand-built *consecutive* frames, which is legitimate - they are the sender's
/// job. What must not be hand-built is the flow control, because that is the output under
/// test.
#[test]
fn fc_is_emitted_after_exactly_block_size_consecutive_frames() {
    const BLOCK_SIZE: u8 = 3;

    let mut reassembler: Reassembler<64> = Reassembler::new(ReassemblerConfig {
        addressing_mode: IsoTpAddressingMode::Normal,
        block_size: BLOCK_SIZE,
        st_min: 0,
    });

    // First frame declaring 60 bytes. The first frame carries 6 and each consecutive frame
    // 7, so 6 + 7*5 = 41 leaves the transfer in progress across the whole loop below and
    // completion never competes with a block boundary.
    let mut first = [0u8; 8];
    first[0] = 0x10;
    first[1] = 60;
    match reassembler.feed(&first).expect("first frame accepted") {
        ReassembleResult::FlowControl { frame, .. } => {
            assert_eq!(
                frame[1], BLOCK_SIZE,
                "the advertised block size is what the receiver will enforce"
            );
        }
        other => panic!("first frame must be answered with flow control, got {other:?}"),
    }

    // Consecutive frames, counting how many arrive between flow controls.
    let mut since_last_fc = 0u8;
    for sequence in 1..=5u8 {
        let mut cf = [0u8; 8];
        cf[0] = 0x20 | (sequence & 0x0F);

        since_last_fc += 1;

        match reassembler.feed(&cf).expect("consecutive frame accepted") {
            ReassembleResult::InProgress => {
                assert!(
                    since_last_fc < BLOCK_SIZE,
                    "no flow control after {since_last_fc} frames, but the block is {BLOCK_SIZE}"
                );
            }
            ReassembleResult::FlowControl { frame, .. } => {
                assert_eq!(
                    since_last_fc, BLOCK_SIZE,
                    "flow control arrived after {since_last_fc} frames, expected {BLOCK_SIZE}"
                );
                assert_eq!(frame[0] & 0xF0, 0x30, "PCI must be flow control");
                assert_eq!(frame[0] & 0x0F, 0x00, "and continue-to-send");
                since_last_fc = 0;
            }
            other => panic!("unexpected result at sequence {sequence}: {other:?}"),
        }
    }
}

/// A block size of zero means "send everything", so no flow control is due mid-transfer.
#[test]
fn block_size_zero_emits_no_further_flow_control() {
    let mut reassembler: Reassembler<64> = Reassembler::new(ReassemblerConfig {
        addressing_mode: IsoTpAddressingMode::Normal,
        block_size: 0,
        st_min: 0,
    });

    let mut first = [0u8; 8];
    first[0] = 0x10;
    first[1] = 40;
    reassembler.feed(&first).expect("first frame accepted");

    for sequence in 1..=4u8 {
        let mut cf = [0u8; 8];
        cf[0] = 0x20 | (sequence & 0x0F);

        assert!(
            matches!(
                reassembler.feed(&cf).expect("consecutive frame accepted"),
                ReassembleResult::InProgress
            ),
            "block size zero must not interrupt the sender at frame {sequence}"
        );
    }
}

// endregion: Flow control - block size

// region: Error cases

#[test]
fn rt_sequence_error() {
    let mut reassembler: Reassembler<64> =
        Reassembler::new(ReassemblerConfig::new(IsoTpAddressingMode::Normal));

    // Valid first frame - total length 20, 6 bytes of data
    let ff = [0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    let result = reassembler.feed(&ff).unwrap();
    assert!(matches!(result, ReassembleResult::FlowControl { .. }));

    // Wrong sequence number - expected 1, got 2
    let cf_wrong = [0x22, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D];
    let err = reassembler.feed(&cf_wrong).unwrap_err();
    assert!(matches!(
        err,
        ace_can::IsoTpError::SequenceError {
            expected: 1,
            actual: 2
        }
    ));
}

#[test]
fn rt_unexpected_consecutive_frame() {
    let mut reassembler: Reassembler<64> =
        Reassembler::new(ReassemblerConfig::new(IsoTpAddressingMode::Normal));

    let cf = [0x21, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let err = reassembler.feed(&cf).unwrap_err();
    assert!(matches!(
        err,
        ace_can::IsoTpError::UnexpectedConsecutiveFrame
    ));
}

#[test]
fn rt_empty_single_frame() {
    let mut reassembler: Reassembler<64> =
        Reassembler::new(ReassemblerConfig::new(IsoTpAddressingMode::Normal));

    let sf = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let err = reassembler.feed(&sf).unwrap_err();
    assert!(matches!(err, ace_can::IsoTpError::EmptySingleFrame));
}

#[test]
fn rt_session_abort() {
    let mut reassembler: Reassembler<64> =
        Reassembler::new(ReassemblerConfig::new(IsoTpAddressingMode::Normal));

    let ff1 = [0x10, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    reassembler.feed(&ff1).unwrap();

    let ff2 = [0x10, 0x0A, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
    let result = reassembler.feed(&ff2).unwrap();
    assert!(matches!(result, ReassembleResult::SessionAborted { .. }));
}

// endregion: Error cases
