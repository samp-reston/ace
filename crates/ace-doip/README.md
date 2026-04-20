# `ace-doip`

DoIP typed message and session layer implementing ISO 13400-2.

**Message layer** — all payload types as structs deriving `FrameCodec`: `RoutingActivationRequest`, `RoutingActivationResponse`, `DiagnosticMessage`, `DiagnosticMessageAck`, `DiagnosticMessageNack`, `VehicleAnnouncementMessage`, `EntityStatusResponse`, `AliveCheckRequest`, `AliveCheckResponse`, and more.

**Session layer** — `ActivationStateMachine` and `ConnectionState` model the per-TCP-connection routing activation lifecycle:

```
Idle → ActivationPending → Active → Deactivated
```

`ActivationAuthProvider` is a hook trait for OEM-specific authentication on `CentralSecurity` (0xFF) activation:

```rust
pub trait ActivationAuthProvider {
    fn authenticate(
        &mut self,
        source_address: u16,
        oem_data: &[u8],
    ) -> Result<(), ActivationDenialReason>;
}
```

`DoipFrameExt` provides semantic accessors on `DoipFrame`:

```rust
frame.validate_header()?;          // checks version, inverse, type, length
frame.payload_type();              // Option<Result<PayloadType, _>>
frame.payload_bytes();             // Option<&[u8]> — bytes after 8-byte header
frame.payload_length_declared();   // length from header bytes 4-7
```
