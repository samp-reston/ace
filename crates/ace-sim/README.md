# `ace-sim`

Deterministic simulation infrastructure for testing protocol state machines.

`ace-sim` is optional infrastructure. Production consumers depend on the individual protocol
crates they need; the simulation crate is for host-side integration, deterministic replay, and
fault-driven validation.

## Simulation layers

- **`SimBus<MAX_DATA, MAX_QUEUED>`** — bounded message delivery with seeded randomness,
  probabilistic loss/reorder/delay/corruption/timeout faults, and deterministic scripted faults.
- **`TcpSimBus`** — connection-oriented delivery with bounded per-pair connections, connection
  refusal/reset/timeout faults, and baseline link transmission latency.
- **`CanSimBus`** — connectionless delivery with CAN bus state, arbitration loss, bit errors,
  bus-off transitions, and baseline CAN transmission latency.
- **`TwoTierRunner`** — composes an outer bus, an inner bus, and bridge nodes without knowing
  whether they represent DoIP, CAN, or another layered transport.
- **`TraceBus` / `TraceReplay`** — record bounded bus operations and replay their observable
  outputs without consulting the RNG or fault configuration.

All clocks are virtual and all buffers are bounded. The crate builds with `no_std`; the optional
`alloc` feature changes the backing storage but not the public capacity contracts.

## Basic deterministic bus

```rust
use ace_core::clock::Duration;
use ace_core::io::NodeAddress;
use ace_sim::bus::SimBus;
use ace_sim::fault::FaultConfig;

let mut bus: SimBus<8, 4> = SimBus::new(7, FaultConfig::none());
let sender = NodeAddress(1);
let receiver = NodeAddress(2);

assert!(bus.send(sender, receiver, &[0x10, 0x03]));
let delivered = bus.tick(Duration::ZERO);
assert_eq!(delivered.len(), 1);
```

## Scripted frame faults

`FaultPlan` is protocol-neutral. It can drop, delay, duplicate, or replace an opaque byte in a
single send attempt. A consumer may use `ReplaceByte` to create a malformed protocol frame, but
`ace-sim` never interprets the byte as an NRC, PCI nibble, or sequence number.

```rust
use ace_sim::fault::{FaultConfig, FaultPlan, ScriptedFault};

let mut plan = FaultPlan::new();
plan.push(ScriptedFault::DropNext).expect("plan capacity");

let mut faults = FaultConfig::none();
faults.script = plan;
```

## Trace and replay

```rust
use ace_core::clock::Duration;
use ace_core::io::NodeAddress;
use ace_sim::fault::FaultConfig;
use ace_sim::trace::TraceBus;

let a = NodeAddress(1);
let b = NodeAddress(2);
let mut bus: TraceBus<8, 4, 16> = TraceBus::new(7, FaultConfig::none());

assert!(
    bus.send(a.clone(), b.clone(), &[1, 2])
        .expect("trace capacity")
);
let _ = bus.tick(Duration::ZERO).expect("trace capacity");

let trace = bus.trace().clone();
let mut replay = trace.replay();
let _ = replay.send(&a, &b, &[1, 2], Duration::ZERO);
let _ = replay.tick(Duration::ZERO).expect("recorded tick");
replay.finish().expect("complete trace");
```

For protocol-specific faulty peers—wrong NRCs, unbounded response-pending chains, stale
ISO-TP sequence numbers, or deliberate session behavior—use a future testkit layer above this
crate rather than adding invalid-behavior modes to production protocol libraries.
