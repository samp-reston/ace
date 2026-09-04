# aced

**Batteries-included entry point for the ACE diagnostics stack.**

`aced` re-exports the full `ace-*` crate family - core codec traits, UDS, DoIP,
CAN/ISO-TP, simulation, client, server, and gateway - behind a single
dependency, with heap-backed (`alloc`) storage enabled **by default**.

If you're building a normal `std` application - a gateway server, a tester
client, a test harness - this is almost certainly the crate you want. If
you're targeting `no_std` or a memory-constrained embedded target, skip this
crate and depend on the individual `ace-*` crates directly instead (see
[No\_std / embedded use](#no_std--embedded-use) below).

## Quick start

```toml
[dependencies]
aced = "0.4"
```

That's it - no `[features]` section required. `alloc` is on by default, so
bounded collections throughout the stack (event queues, frame buffers,
outboxes) are backed by the heap instead of large inline arrays, avoiding the
stack-overflow issues that come from oversized `const` capacity generics on
inline storage.

```rust
use aced::uds::UdsClient;
use aced::doip::DoipGateway;

// ace::core, ace::macros, ace::proto, ace::can, ace::sim,
// ace::client, ace::server, and ace::gateway are all available the same way.
```

## Why alloc-by-default here, but not in the underlying crates

The individual `ace-*` crates default to `alloc` **off**, because they're
designed to also serve genuinely `no_std`, no-allocator embedded targets -
that guarantee only holds if nothing forces allocation on them. `aced` exists
specifically to be the opposite: a single, opinionated dependency for
`std` consumers who don't want to think about feature forwarding across ten
crates just to get sensible defaults. The decision to default to `alloc`
is made once, here, and nowhere else in the workspace.

## Feature flags

| Feature | Default | Effect |
|---|---|---|
| `alloc` | ✅ on | Enables heap-backed storage (`alloc::vec::Vec`-based) across every re-exported crate, instead of inline `heapless::Vec` storage. |

To opt out and fall back to heapless/no_std-style storage while still using
this crate (e.g. to keep your own binary's dependency list short while still
targeting a constrained environment):

```toml
[dependencies]
aced = { version = "0.4", default-features = false }
```

## Crates in this workspace

`aced` re-exports each of the following under a matching module name
(`ace::core`, `ace::uds`, etc.):

| Module | Crate | Description |
|---|---|---|
| `ace::core` | [`ace-core`](https://crates.io/crates/ace-core) | Foundation layer - `FrameRead`, `FrameWrite`, `Writer` codec traits everything else builds on. |
| `ace::macros` | [`ace-macros`](https://crates.io/crates/ace-macros) | Proc-macro crate providing `FrameCodec`, which derives `FrameRead`/`FrameWrite` for structs and enums. |
| `ace::proto` | [`ace-proto`](https://crates.io/crates/ace-proto) | Raw, protocol-agnostic frame wrappers - UDS, DoIP, and CAN frames, with mutable variants. |
| `ace::can` | [`ace-can`](https://crates.io/crates/ace-can) | ISO-TP implementation - reassembler and segmenter bridging DoIP UDS payloads to CAN frames. |
| `ace::sim` | [`ace-sim`](https://crates.io/crates/ace-sim) | Deterministic simulation infrastructure for reproducibly testing protocol state machines. |
| `ace::uds` | [`ace-uds`](https://crates.io/crates/ace-uds) | UDS typed message layer implementing ISO 14229-1. |
| `ace::doip` | [`ace-doip`](https://crates.io/crates/ace-doip) | DoIP typed message and session layer implementing ISO 13400-2. |
| `ace::client` | [`ace-client`](https://crates.io/crates/ace-client) | UDS tester client state machine. |
| `ace::server` | [`ace-server`](https://crates.io/crates/ace-server) | UDS ECU server state machine - session management, security access, timing, periodic DIDs, NRCs. |
| `ace::gateway` | [`ace-gateway`](https://crates.io/crates/ace-gateway) | DoIP gateway, ISO-TP bridge node, and DoIP tester. |

Each of these crates is also independently published and can be depended on
directly.

## No\_std / embedded use

`aced` is not the right dependency for `no_std` or allocator-free targets.
Depend on the individual crates you need instead, with `default-features =
false`:

```toml
[dependencies]
ace-core = { version = "0.4", default-features = false }
ace-uds  = { version = "0.4", default-features = false }
```

In this mode, bounded collections are backed by `heapless::Vec`, with
capacity fixed at compile time via `const` generics - no heap allocator
required, at the cost of each type reserving its full worst-case capacity
inline. See each crate's own README for its `no_std` support details.

## License

Licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
