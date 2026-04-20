# `ace-macros`

Proc-macro crate. Provides `#[derive(FrameCodec)]` which generates `FrameRead` and `FrameWrite` impls for structs and enums.

```rust
#[derive(Clone, Debug, PartialEq, Eq, FrameCodec)]
#[frame(error = UdsError)]
pub struct DiagnosticSessionControlRequest {
    pub session_type: DiagnosticSessionType,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, FrameCodec)]
#[frame(error = UdsError)]
pub enum DiagnosticSessionType {
    #[frame(id = 0x01)]
    DefaultSession,
    #[frame(id = 0x02)]
    ProgrammingSession,
    #[frame(id_pat = "0x05..=0x3F")]
    ISOSAEReserved(u8),
}
```

Field attributes:
- `#[frame(id = 0xNN)]` — discriminant for unit and newtype enum variants
- `#[frame(id_pat = "...")]` — pattern for catchall variants carrying a raw `u8`
- `#[frame(length = expr)]` — fixed byte count for slice fields
- `#[frame(read_all)]` — consume all remaining bytes (trailing `&[u8]` fields)
- `#[frame(skip)]` — exclude from encode/decode, initialise with `Default`
