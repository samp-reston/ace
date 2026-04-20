# ace - Automotive Communication Engine

A `no_std`-first Rust workspace for automotive diagnostic communication. Implements ISO 14229-1 (UDS), ISO 13400-2 (DoIP), and ISO 15765-2 (ISO-TP) with a deterministic simulation layer for property-based testing.

---

## Design principles

**`no_std` by default.** Every protocol crate compiles on bare-metal targets with no heap allocator. `alloc` and `std` are opt-in features. All buffers are statically sized via const generics.

**Zero-copy.** Frame parsing operates on borrowed byte slices. No allocation occurs in the decode path. Payload references carry the lifetime of the original receive buffer.

**Deterministic simulation first.** Protocol state machines are designed from the ground up to be driven by injected time and injected randomness. The same seed always reproduces the same execution. Bugs found in simulation are real bugs that will occur in production.

**Transport agnostic.** `UdsServer` and `UdsClient` know nothing about CAN or Ethernet. The transport layer - ISO-TP over CAN, DoIP over TCP — is wired externally. This makes both layers independently testable and independently replaceable.

---

## Workspace layout

```
ace/
├── ace-can         - ISO-TP reassembler and segmenter (ISO 15765-2)
├── ace-client      - UDS tester client state machine
├── ace-core        - Codec traits, error types, primitive impls
├── ace-doip        - DoIP typed message and session layer (ISO 13400-2)
├── ace-gateway     - DoIP gateway, ISO-TP bridge, DoIP tester
├── ace-macros      - Proc-macro derives: FrameRead, FrameWrite, FrameCodec
├── ace-proto       - Raw frame types: UdsFrame, DoipFrame, CAN frames
├── ace-server      - UDS ECU server state machine
├── ace-sim         - Deterministic simulation infrastructure
└── ace-tests       - DST property tests for the full stack
├── ace-transport   - Production OS transport (std, TCP/UDP)
├── ace-uds         - UDS typed message layer (ISO 14229-1)
```

---

## Crate reference

### `ace-core`

Foundation layer. Defines the three codec traits that everything else builds on:

- `FrameRead<'a>` - zero-copy decode from a `&mut &'a [u8]` cursor
- `FrameWrite` - encode into a `Writer` (either `&mut [u8]` or `BytesMut`)
- `Writer` - sealed trait abstracting alloc and no-alloc write targets

Also provides `DiagError`, `AddressMode`, `DiagnosticAddress`, and the `FrameIter<'a, T>` lazy iterator for variable-length repeated fields.

```toml
[dependencies]
ace-core = { path = "../ace-core", default-features = false }
```

---

### `ace-macros`

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
- `#[frame(id = 0xNN)]` - discriminant for unit and newtype enum variants
- `#[frame(id_pat = "...")]` - pattern for catchall variants carrying a raw `u8`
- `#[frame(length = expr)]` - fixed byte count for slice fields
- `#[frame(read_all)]` - consume all remaining bytes (trailing `&[u8]` fields)
- `#[frame(skip)]` - exclude from encode/decode, initialise with `Default`

---

### `ace-proto`

Raw frame wrappers with no protocol knowledge. Provides `UdsFrame<'a>`, `UdsFrameMut<'a>`, `DoipFrame<'a>`, `DoipFrameMut<'a>`, and CAN frame types (`CanFrame`, `CanFrameMut`, `CanFdFrame`, `CanFdFrameMut`).

These types wrap byte slices and provide structural access - length, index, iteration. Protocol semantics are added by extension traits in `ace-uds` and `ace-doip`.

---

### `ace-uds`

UDS typed message layer implementing ISO 14229-1.

Provides all service request and response types as structs and enums deriving `FrameCodec`. Also provides:

- `UdsFrameExt` - semantic accessors on `UdsFrame`: `service_identifier()`, `sub_function_value()`, `is_suppressed()`, `payload()`, `is_negative_response()`, `negative_response_code()`
- `ServiceIdentifier` enum - all ISO 14229-1 SIDs with `has_sub_function()` helper

```rust
use ace_uds::ext::UdsFrameExt;
use ace_proto::uds::UdsFrame;

let frame = UdsFrame::from_slice(data);
let sid = frame.service_identifier();          // Option<ServiceIdentifier>
let suppressed = frame.is_suppressed();        // bool
let payload = frame.payload();                 // &[u8] after SID byte
```

---

### `ace-doip`

DoIP typed message and session layer implementing ISO 13400-2.

**Message layer** - all payload types as structs deriving `FrameCodec`: `RoutingActivationRequest`, `RoutingActivationResponse`, `DiagnosticMessage`, `DiagnosticMessageAck`, `DiagnosticMessageNack`, `VehicleAnnouncementMessage`, `EntityStatusResponse`, `AliveCheckRequest`, `AliveCheckResponse`, and more.

**Session layer** - `ActivationStateMachine` and `ConnectionState` model the per-TCP-connection routing activation lifecycle:

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
frame.payload_bytes();             // Option<&[u8]> - bytes after 8-byte header
frame.payload_length_declared();   // length from header bytes 4-7
```

---

### `ace-can`

ISO-TP implementation (ISO 15765-2). Provides the reassembler and segmenter used by `ace-gateway`'s `IsoTpNode` to bridge DoIP UDS payloads to CAN frames.

**Design:** addressing mode (Normal / Extended / Mixed) is a caller concern. The reassembler and segmenter operate on pure PCI bytes - callers strip/prepend the address byte at the transport boundary.

```rust
// Segmenter - owns its payload buffer, no lifetime, no unsafe
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

---

### `ace-server`

UDS ECU server state machine (ISO 14229-1 ECU side).

Handles all session management, security access state, P2/S3 timing, periodic DID scheduling, and NRC construction internally. The application provides data and actions via two traits:

**`ServerHandler`** - required hooks for service handling:

```rust
pub trait ServerHandler {
    type Error: NrcError;

    // Required
    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn write_did(&mut self, did: u16, data: &[u8]) -> Result<(), Self::Error>;
    fn ecu_reset(&mut self, reset_type: u8) -> Result<(), Self::Error>;

    // Optional - default returns NRC 0x11 serviceNotSupported
    fn routine_control(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn communication_control(&mut self, ...) -> Result<(), Self::Error> { Err(...) }
    fn io_control(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn request_download(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn transfer_data(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn request_transfer_exit(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
    fn request_file_transfer(&mut self, ...) -> Result<usize, Self::Error> { Err(...) }
}
```

**`SecurityProvider`** - seed generation and key validation:

```rust
pub trait SecurityProvider {
    fn generate_seed(&mut self, level: u8, buf: &mut [u8]) -> Result<usize, SecurityError>;
    fn validate_key(&self, level: u8, seed: &[u8], key: &[u8]) -> Result<(), SecurityError>;
}
```

**`ServerConfig`** - ODX-style ECU description built with a fluent builder:

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

The server is driven by three methods - identical in simulation and production:

```rust
server.handle(&src_addr, &raw_uds_bytes, now)?;  // process inbound frame
server.tick(now)?;                                 // drive timers
server.drain_outbox(&mut out);                     // collect responses
```

---

### `ace-client`

UDS tester client state machine (ISO 14229-1 tester side).

A dumb request/response pipe. Sends raw UDS frames, emits `ClientEvent`s as responses arrive. Tracks P2/P2* timeouts per pending request. Session state, security state, and retry logic are the caller's responsibility.

```rust
let mut client = UdsClient::<1>::new(config, address);

// Send a request
client.request(&[0x10, 0x03], now)?;

// After ticking, drain events
for event in client.drain_events() {
    match event {
        ClientEvent::PositiveResponse { sid, data } => { /* ... */ }
        ClientEvent::NegativeResponse { sid, nrc }  => { /* ... */ }
        ClientEvent::ResponsePending  { sid }        => { /* extended timeout active */ }
        ClientEvent::Timeout          { sid }        => { /* no response in time */ }
        ClientEvent::PeriodicData     { did, data }  => { /* periodic DID data */ }
        ClientEvent::Unsolicited      { data }        => { /* unmatched frame */ }
    }
}
```

Periodic DID subscriptions control classification of `0xF2xx` response frames:

```rust
client.subscribe_periodic(0x90);   // DID 0xF290 low byte
client.unsubscribe_periodic(0x90);
```

---

### `ace-sim`

Deterministic simulation infrastructure. Everything needed to test protocol state machines reproducibly.

**`SimBus<N, Q>`** - message delivery with fault injection. Seeded RNG makes every run reproducible. Configurable faults: message loss, reorder, delay, corruption, timeout.

**`TcpSimBus<N, Q>`** - wraps `SimBus` and adds TCP connection state. The bus owns connection state — nodes don't track it. TCP fault injection: connection refused, mid-session reset, connect timeout.

**`CanSimBus<N, Q>`** - wraps `SimBus` and adds CAN bus state (Active / ErrorPassive / BusOff). CAN fault injection: arbitration loss, bit error, bus-off.

**`SimNode<N, Q>`** - trait for nodes on the simulation buses:

```rust
pub trait SimNode<const N: usize, const Q: usize> {
    type Error: core::fmt::Debug;
    fn address(&self) -> &NodeAddress;
    fn handle(&mut self, src: &NodeAddress, data: &[u8], now: Instant) -> Result<(), Self::Error>;
    fn tick(&mut self, now: Instant) -> Result<(), Self::Error>;
    fn drain_outbox(&mut self, out: &mut Vec<(NodeAddress, Vec<u8, N>), Q>) -> usize;
}
```

**`SimNodeErased<N, Q>`** - object-safe version with errors swallowed internally, enabling heterogeneous slices of different node types.

**`SimRunner<N, Q>`** - drives `SimNodeErased` slices over a `SimBus`.

**`TcpSimRunner<N, Q>`** - drives nodes over a `TcpSimBus`, additionally delivering `TcpEvent`s to nodes implementing `TcpEventHandler`.

**`CanSimRunner<N, Q>`** - drives nodes over a `CanSimBus`, additionally delivering `CanEvent`s to nodes implementing `CanEventHandler`.

**`FaultConfig`** - three presets: `none()`, `light()`, `chaos()`. All probabilities are `(numerator, denominator)` pairs for reproducibility.

---

### `ace-gateway`

DoIP gateway, ISO-TP bridge node, and DoIP tester.

**`DoipGateway<A, MAX_TESTERS, BUF>`** - gateway state machine. Translates DoIP frames from TCP into UDS bytes on CAN, and CAN responses back into DoIP frames. Has two faces — `handle_tcp` and `handle_can` — because it spans two buses. Routing table maps DoIP logical addresses to CAN IDs.

**`IsoTpNode<N>`** - bridges raw UDS bytes to ISO-TP CAN frames. Two independent segmenters (request and response directions) to handle concurrent multi-frame exchanges. Key methods: `handle_from_gateway(uds_data)`, `handle_uds_response(uds_data)`, `handle_from_ecu(can_frame)`.

**`DoipTester<MAX_CONNECTIONS, MAX_TARGETS>`** - models a physical DoIP tester device. Owns multiple `DoipConnection`s (one per TCP connection). Each connection addresses multiple ECUs simultaneously via `target_address`. P2/P2* timeouts are learned dynamically from `DiagnosticSessionControlResponse`. Events are tagged `(ConnectionId, TargetId, DoipTesterEvent)`.

```rust
let mut tester = DoipTester::<4, 8>::new(0x0E00, NodeAddress(0x0E00));

// Open a connection to a gateway
let conn = tester.open_connection(DoipConnectionConfig::new(0x0E80))?;

// After TCP connects (TcpSimBus::connect or real TcpStream):
// tester.on_tcp_event(&TcpEvent::ConnectionEstablished { .. }, now);
// → automatically sends RoutingActivationRequest

// Send UDS to ECU 0x0001 on that connection
tester.request(conn, 0x0001, &[0x10, 0x03], now)?;

// Drain events
for (conn_id, target_id, event) in tester.drain_events() {
    // ...
}
```

Node profiles accumulate from UDP announcements:

```rust
if let Some(profile) = tester.profile(0x0E80) {
    println!("VIN: {:?}", profile.vin);
}
```

**`GatewayConfig`** builder:

```rust
let config = GatewayConfig::new(0x0E80)
    .with_tester(0x0E00)
    .with_node(CanNodeEntry {
        logical_address:   0x0001,
        request_can_id:    0x7E0,
        response_can_id:   0x7E8,
        functional_can_id: 0x7DF,
    });
```

---

### `ace-transport`

Production OS transport layer. The only `std`-required crate in the workspace.

**`DoipVehicleDriver`** - wraps `DoipTester` with real `TcpStream` and `UdpSocket` transport. Background threads handle blocking I/O. The main application calls `tick()` in a loop.

```rust
let mut driver = DoipVehicleDriver::new(VehicleDriverConfig::default());

// Optional: discover gateways via UDP multicast
let gateways = discover_gateways(&DiscoveryConfig {
    protocol_version: DiscoveryProtocolVersion::Iso13400_2012,
    ..DiscoveryConfig::default()
})?;

// Connect to a gateway
let conn = driver.connect("192.168.1.10:13400".parse()?, DoipConnectionConfig::new(0x0E80))?;

// Main loop
loop {
    driver.tick();
    for (_, target, event) in driver.drain_events() {
        // handle events
    }
    std::thread::sleep(Duration::from_millis(1));
}
```

---

### `ace-tests`

Workspace-level DST property test crate. Uses `ace-sim` to run the full stack deterministically across hundreds of seeds and three fault regimes.

Run all tests:
```
cargo test -p ace-tests
```

Run a specific service group:
```
cargo test -p ace-tests dst::session
cargo test -p ace-tests dst::security
cargo test -p ace-tests dst::data
cargo test -p ace-tests dst::periodic
cargo test -p ace-tests dst::doip
```

Run with output visible (shows which seed failed):
```
cargo test -p ace-tests -- --nocapture --test-threads=1
```

Reproduce a specific failing seed - find the seed in the test output then hardcode it temporarily:
```rust
#[test]
fn p1_session_control_extended_no_faults() {
    for seed in [42u64] { // narrowed to failing seed
        // ...
    }
}
```

---

## Using the system

### Scenario 1 - UDS ECU in simulation (no transport)

```rust
use ace_server::{ServerConfig, SessionConfig, ServiceConfig, DidConfig,
                  SecurityLevelConfig, UdsServer, BuiltinNrc};
use ace_server::handler::ServerHandler;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_sim::clock::{Duration, Instant};
use ace_sim::io::NodeAddress;

// 1. Define your application handler
struct MyHandler { vin: [u8; 17] }

impl ServerHandler for MyHandler {
    type Error = BuiltinNrc;

    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, BuiltinNrc> {
        match did {
            0xF190 => {
                let len = self.vin.len().min(buf.len());
                buf[..len].copy_from_slice(&self.vin[..len]);
                Ok(len)
            }
            _ => Err(BuiltinNrc::RequestOutOfRange),
        }
    }
    fn write_did(&mut self, _did: u16, _data: &[u8]) -> Result<(), BuiltinNrc> {
        Err(BuiltinNrc::RequestOutOfRange)
    }
    fn ecu_reset(&mut self, _reset_type: u8) -> Result<(), BuiltinNrc> {
        Ok(())
    }
}

// 2. Define your security provider
struct MySecurityProvider;

impl SecurityProvider for MySecurityProvider {
    fn generate_seed(&mut self, level: u8, buf: &mut [u8]) -> Result<usize, SecurityError> {
        buf[0] = level ^ 0xAB;
        Ok(1)
    }
    fn validate_key(&self, _level: u8, seed: &[u8], key: &[u8]) -> Result<(), SecurityError> {
        if key.first() == Some(&(seed[0] ^ 0xFF)) { Ok(()) }
        else { Err(SecurityError::InvalidKey) }
    }
}

// 3. Build the server config
let config = ServerConfig::new(0x0001, 0x7DF)
    .with_session(SessionConfig::default_session())
    .with_session(SessionConfig::extended_session())
    .with_service(ServiceConfig::new(0x22, &[0x01, 0x02, 0x03]))
    .with_service(ServiceConfig::new(0x3E, &[0x01, 0x02, 0x03]))
    .with_did(DidConfig::read_only(0xF190, &[0x01, 0x02, 0x03]));

// 4. Create the server
let mut server = UdsServer::new(
    config,
    MyHandler { vin: *b"TESTVIN1234567890" },
    MySecurityProvider,
    NodeAddress(0x0001),
);

// 5. Drive it with raw UDS bytes
let now = Instant::ZERO;
server.handle(&NodeAddress(0x0E00), &[0x22, 0xF1, 0x90], now).unwrap();
server.tick(now).unwrap();

let mut outbox = heapless::Vec::<_, 16>::new();
server.drain_outbox(&mut outbox);
// outbox[0].1 contains [0x62, 0xF1, 0x90, ...VIN bytes...]
```

---

### Scenario 2 - Full UDS round-trip in simulation

```rust
use ace_tests::fixtures::doip::DoipDstScenario;

let mut s = DoipDstScenario::baseline(0); // seed 0, no faults
s.connect();
s.tick_n(50); // activate routing

assert!(s.is_activated());

s.request_default(&[0x10, 0x03]).unwrap(); // DSC extended
s.tick_n(500);

let events = s.drain_events();
// find PositiveResponse { sid: 0x10, .. }
```

---

### Scenario 3 - Multi-ECU multi-gateway simulation

```rust
use ace_tests::fixtures::doip::{
    DoipDstScenarioBuilder, GatewayNodeConfig, EcuNodeConfig,
};

let mut s = DoipDstScenarioBuilder::new(42)
    .with_gateway(
        GatewayNodeConfig::new(0x0E80, 0x0E00)
            .with_ecu(EcuNodeConfig::new(0x0001, 0x7E0, 0x7E8, 0x7DF))
            .with_ecu(EcuNodeConfig::new(0x0002, 0x7E2, 0x7EA, 0x7DF))
    )
    .add_gateway(
        GatewayNodeConfig::new(0x0E81, 0x0E00)
            .with_ecu(EcuNodeConfig::new(0x0003, 0x7E4, 0x7EC, 0x7DF))
    )
    .build();

s.connect();
s.tick_n(50);

let gw1 = s.gateways[0].conn_id;
let gw2 = s.gateways[1].conn_id;

// Talk to ECU 0x0001 on gateway 1
s.request(gw1, 0x0001, &[0x22, 0xF1, 0x90]).unwrap();

// Talk to ECU 0x0003 on gateway 2 simultaneously
s.request(gw2, 0x0003, &[0x3E, 0x80]).unwrap(); // TesterPresent suppressed

s.tick_n(500);
```

---

### Scenario 4 - Real vehicle connection

```rust
use ace_transport::doip_vehicle_driver::{DoipVehicleDriver, VehicleDriverConfig};
use ace_gateway::tester::{DoipConnectionConfig, DoipTesterEvent};
use ace_client::event::ClientEvent;

let mut driver = DoipVehicleDriver::new(VehicleDriverConfig::default());

let conn = driver.connect(
    "192.168.1.10:13400".parse().unwrap(),
    DoipConnectionConfig::new(0x0E80),
)?;

loop {
    driver.tick();

    for (_, _, event) in driver.drain_events() {
        match event {
            DoipTesterEvent::ActivationSucceeded => {
                driver.request(conn, 0x0001, &[0x10, 0x03]).unwrap();
            }
            DoipTesterEvent::Uds(ClientEvent::PositiveResponse { sid: 0x10, .. }) => {
                driver.request(conn, 0x0001, &[0x22, 0xF1, 0x90]).unwrap();
            }
            DoipTesterEvent::Uds(ClientEvent::PositiveResponse { sid: 0x22, data }) => {
                println!("VIN: {}", core::str::from_utf8(&data[2..]).unwrap_or("?"));
                break;
            }
            _ => {}
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(1));
}
```

---

### Scenario 5 - Running DST under fault injection

```rust
use ace_sim::fault::FaultConfig;
use ace_sim::tcp::TcpFaultConfig;
use ace_sim::can_bus::CanFaultConfig;
use ace_tests::fixtures::doip::{DoipDstScenarioBuilder, GatewayNodeConfig, EcuNodeConfig};

// Run the same property under 100 seeds at chaos fault level
for seed in 0..100u64 {
    let mut s = DoipDstScenarioBuilder::new(seed)
        .with_tcp_faults(TcpFaultConfig::chaos())
        .with_can_faults(CanFaultConfig::chaos())
        .build();

    s.connect();
    s.tick_n(10_000);

    // Property: no pending requests remain - every exchange resolved
    for gw in &s.gateways {
        for ecu in &gw.ecus {
            let pending = s.tester.connection_pending_count(gw.conn_id, ecu.logical_address);
            assert_eq!(pending, 0, "seed {seed}: pending request not resolved");
        }
    }
}
```

---

## Feature flags

| Crate | `alloc` | `std` |
|---|---|---|
| `ace-core` | `bytes::BytesMut` Writer impl | - |
| `ace-macros` | - | — |
| `ace-proto` | - | — |
| `ace-uds` | inherits from `ace-core` | - |
| `ace-doip` | inherits | - |
| `ace-can` | inherits | - |
| `ace-server` | inherits | - |
| `ace-client` | inherits | - |
| `ace-sim` | - | — |
| `ace-gateway` | inherits | - |
| `ace-transport` | required | required |

For embedded targets use `default-features = false` on all crates except `ace-transport`.

---

## Crate dependency graph

```
ace-transport
    └── ace-gateway
            ├── ace-doip
            │       └── ace-core, ace-proto, ace-macros
            ├── ace-can
            │       └── ace-core
            ├── ace-client
            │       └── ace-core, ace-uds, ace-sim
            └── ace-sim
                    └── (no ace dependencies)

ace-server
    └── ace-core, ace-uds, ace-sim

ace-tests (dev)
    └── ace-server, ace-client, ace-gateway, ace-sim, ace-can, ace-doip

ace-sim
    └── ace-core
```
