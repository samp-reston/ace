# `ace-core`

Foundation layer. Defines the three codec traits that everything else builds on:

- `FrameRead<'a>` — zero-copy decode from a `&mut &'a [u8]` cursor
- `FrameWrite` — encode into a `Writer` (either `&mut [u8]` or `BytesMut`)
- `Writer` — sealed trait abstracting alloc and no-alloc write targets

Also provides `DiagError`, `AddressMode`, `DiagnosticAddress`, and the `FrameIter<'a, T>` lazy iterator for variable-length repeated fields.

```toml
[dependencies]
ace-core = { path = "../ace-core", default-features = false }
```
