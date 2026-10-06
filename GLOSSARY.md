# Glossary

Common terms and fixed values used across this workspace. Design standards that
use these terms are in [`CODING_STANDARDS.md`](CODING_STANDARDS.md).

## Protocol terms

| Term | Meaning |
|---|---|
| **UDS** | Unified Diagnostic Services (ISO 14229-1). The request/response diagnostic protocol. |
| **DoIP** | Diagnostics over IP (ISO 13400-2). Carries UDS over TCP/UDP. |
| **ISO-TP** | ISO 15765-2 transport protocol. Segments/reassembles UDS payloads over CAN frames. |
| **CAN-FD** | CAN with Flexible Data-rate. The link layer under ISO-TP. |
| **Entity** | The generic DoIP peer. A Gateway Entity or a Node Entity (see `node_type`). "Entity", never "Gateway", is the generic term. |
| **Gateway Entity** | A DoIP Entity that routes to further ECUs; only its successful activation triggers the follow-up broadcast VIR. |
| **Node Entity** | A DoIP Entity that is an endpoint, not a router. |
| **Routing Activation** | DoIP handshake a tester performs with an Entity before diagnostics. |
| **VIR / VAM** | Vehicle Identification Request / Announcement Message — DoIP discovery. |
| **Physical addressing** | A request targeted at one server's own address. |
| **Functional addressing** | A broadcast request to a group; servers suppress the "not implemented" NRCs on this path. |
| **SID** | Service Identifier — the first byte of a UDS request (e.g. `0x22` ReadDataByIdentifier). |
| **DID** | Data Identifier — a 2-byte addressable data item. |
| **Sub-function** | The byte after the SID on services that take one. |
| **NRC** | Negative Response Code — the reason byte in a `0x7F` negative response. |
| **SecurityAccess** | UDS `0x27` seed/key unlock. Distinct from Authentication (`0x29`). |
| **Seed / key** | The challenge the server issues and the response the tester computes to unlock a security level. |
| **P2 / P2\*** | Response timing budgets: P2 is the normal deadline, P2\* the extended deadline after a `0x78`. |
| **S3** | Session timeout — idle time after which the server falls back to the default session. |
| **`0x78`** | requestCorrectlyReceivedResponsePending — "accepted, still working". Commits the server to a final response. |
| **suppressPosRspMsgIndicationBit** | Request bit asking the server to withhold a positive response. |

## Project terms

| Term | Meaning |
|---|---|
| **the suite** | The `ace-*` crates together. A library suite for *building* a diagnostic stack — not a product (see `CODING_STANDARDS.md` §1). |
| **demonstration** | The deterministic vehicle simulation delivered via `ace-tests`; proves the suite is sufficient, and is not shipped as a product. |
| **`ace-api` / `aced`** | The facade crate is published on crates.io as `ace-api`; its library/import name is `aced` (`use aced::…`). Deliberate — see `crates/ace-api/README.md`. |
| **`DiagnosticNode`** | Core trait every node (server/client/gateway/tester) implements: `address`, `handle`, `tick`, `drain_outbox`. |
| **`InboundFrame`** | A delivered frame carrying `src`, `dst`, and `data` — the unit a node filters on by identity. |
| **`NodeAddress`** | A node's logical (UDS) address; the transport maps its own addressing onto this space. |
| **`CapacityError`** | The typed error returned when a bounded buffer cannot accept more. |
| **`SimBus` / `TraceBus`** | The deterministic in-memory bus; `TraceBus`/`TraceReplay` record and replay a run below protocol knowledge. |
| **`FaultPlan`** | A bounded, protocol-neutral script of frame faults (Drop / Delay / Duplicate / ReplaceByte). |

## Crate roles

| Crate | Role | Published |
|---|---|---|
| `ace-core` | Foundation: codec traits, bounded `Vec<T,N>`, clock, RNG, IO types, `DiagnosticNode`, `CodecError`. | yes |
| `ace-macros` | `FrameCodec` derive (proc-macro only). | yes |
| `ace-proto` | Raw protocol-agnostic frame wrappers + addressing classification. | yes |
| `ace-can` | ISO-TP reassembler / segmenter. | yes |
| `ace-uds` | Typed UDS message layer (ISO 14229-1). | yes |
| `ace-doip` | Typed DoIP message + session layer (ISO 13400-2). | yes |
| `ace-server` | UDS ECU server state machine. | yes |
| `ace-client` | UDS tester client state machine. | yes |
| `ace-gateway` | DoIP gateway, ISO-TP bridge node, DoIP tester. | yes |
| `ace-sim` | Deterministic simulation (bus, clock, RNG, trace/replay, faults). | yes |
| `ace-api` | Alloc-backed facade re-exporting the suite. | yes (as `ace-api`) |
| `ace-tests` | The demonstration; composes the crates directly. | no (`publish = false`) |
| `ace-transport` | Host-side transport/driver glue. | no (`publish = false`) |

## Fixed values

| Value | Setting |
|---|---|
| MSRV | Rust **1.88** (validated against the real dependency graph, not the edition floor). |
| Edition | 2024 |
| Workspace version | **0.6.0** (shared across all crates) |
| Published baseline | **0.5.5** on crates.io (the `cargo-semver-checks` baseline) |
| Embedded target | `thumbv7em-none-eabihf`, `--no-default-features` |
| Canonical check | `sh ci/verify-matrix.sh` |
| Tool pins | cargo-hack `0.6.45`, cargo-msrv `0.19.3`, cargo-semver-checks `0.47.0`, cargo-release `1.1.2` |
