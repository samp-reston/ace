# `ace-uds`

UDS typed message layer implementing ISO 14229-1.

Provides all service request and response types as structs and enums deriving `FrameCodec`. Also provides:

- `UdsFrameExt` — semantic accessors on `UdsFrame`: `service_identifier()`, `sub_function_value()`, `is_suppressed()`, `payload()`, `is_negative_response()`, `negative_response_code()`
- `ServiceIdentifier` enum — all ISO 14229-1 SIDs with `has_sub_function()` helper

```rust
use ace_uds::ext::UdsFrameExt;
use ace_proto::uds::UdsFrame;

let frame = UdsFrame::from_slice(data);
let sid = frame.service_identifier();          // Option<ServiceIdentifier>
let suppressed = frame.is_suppressed();        // bool
let payload = frame.payload();                 // &[u8] after SID byte
```
