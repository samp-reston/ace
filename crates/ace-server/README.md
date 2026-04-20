# `ace-server`

UDS ECU server state machine (ISO 14229-1 ECU side).

Handles all session management, security access state, P2/S3 timing, periodic DID scheduling, and NRC construction internally. The application provides data and actions via two traits:

**`ServerHandler`** — required hooks for service handling:

```rust
pub trait ServerHandler {
    type Error: NrcError;

    // Required
    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn write_did(&mut self, did: u16, data: &[u8]) -> Result<(), Self::Error>;
    fn ecu_reset(&mut self, reset_type: u8) -> Result<(), Self::Error>;

    // Optional — default returns NRC 0x11 serviceNotSupported
    fn routine_control(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn communication_control(&mut self, ...) -> Result<(), Self::Error> { Err(...) }
    fn io_control(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn request_download(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn transfer_data(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn request_transfer_exit(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn request_file_transfer(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
}
```

**`SecurityProvider`** — seed generation and key validation:

```rust
pub trait SecurityProvider {
    fn generate_seed(&mut self, level: u8, buf: &mut [u8]) -> Result<usize, SecurityError>;
    fn validate_key(&self, level: u8, seed: &[u8], key: &[u8]) -> Result<(), SecurityError>;
}
```

**`ServerConfig`** — ODX-style ECU description built with a fluent builder:

```rust
let config = ServerConfig::new(physical_address: 0x0001, functional_address: 0x7DF)
    .with_session(SessionConfig::default_session())
    .with_session(SessionConfig::extended_session())
    .with_service(ServiceConfig::new(0x22, &[0x01, 0x02, 0x03]))
    .with_service(ServiceConfig::secured(0x2E, &[0x03], security_level: 1))
    .with_did(DidConfig::read_only(0xF190, &[0x01, 0x02, 0x03]))
    .with_did(DidConfig::read_write(0xF120, &[0x03], &[0x03]).secured(1))
    .with_security_level(SecurityLevelConfig {
        level: 0x01,
        max_attempts: 3,
        lockout_duration: Duration::from_millis(10_000),
        seed_length: 4,
        key_length: 4,
    });
```

The server is driven by three methods — identical in simulation and production:

```rust
server.handle(&src_addr, &raw_uds_bytes, now)?;  // process inbound frame
server.tick(now)?;                                 // drive timers
server.drain_outbox(&mut out);                     // collect responses
```
