# `ace-can`

ISO-TP implementation (ISO 15765-2). Provides the reassembler and segmenter used by `ace-gateway`'s `IsoTpNode` to bridge DoIP UDS payloads to CAN frames.

**Design:** addressing mode (Normal / Extended / Mixed) is a caller concern. The reassembler and segmenter operate on pure PCI bytes — callers strip/prepend the address byte at the transport boundary.

```rust
// Segmenter — owns its payload buffer, no lifetime, no unsafe
let mut seg = Segmenter::<4096>::new(SegmenterConfig::classic(Normal));
seg.start(&uds_payload)?;

let mut out = [0u8; 8];
loop {
    match seg.next_frame(&mut out)? {
        SegmentResult::Frame { len } => { /* put out[..len] on CAN bus */ }
        SegmentResult::Complete      => break,
        SegmentResult::WaitForFlowControl => {
            // wait for FC from receiver then call seg.handle_flow_control(fc)
        }
    }
}
```

```rust
// Reassembler
let mut rsm = Reassembler::<4096>::new(ReassemblerConfig::new(Normal));
match rsm.feed(&can_frame_bytes)? {
    ReassembleResult::Complete { len }     => { /* rsm.message(len) */ }
    ReassembleResult::FlowControl { .. }   => { /* send FC back */ }
    ReassembleResult::InProgress           => {}
    ReassembleResult::SessionAborted { .. } => {}
}
```
