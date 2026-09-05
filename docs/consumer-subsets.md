# Consumer subsets

`ace` is a suite of libraries for building a diagnostic stack, not a stack itself.
Different consumers depend on different, overlapping subsets of it. This document
is the authoritative statement of those subsets, so an integrator can pick a
minimal dependency set instead of taking the whole workspace.

Every closure and command below was verified, not inferred.

## The subsets

| Subset | Depend on | Full closure | `no_std`, no allocator |
|---|---|---|---|
| **Bare-metal ECU server** | `ace-server` | `ace-core`, `ace-proto`, `ace-uds`, `ace-macros`¹ | yes |
| **Tester client** | `ace-client` | `ace-core`, `ace-proto`, `ace-uds`, `ace-macros`¹ | yes |
| **Gateway / bridge** | `ace-gateway` | `ace-can`, `ace-client`, `ace-core`, `ace-doip`, `ace-proto`, `ace-server`, `ace-uds`, `ace-macros`¹ | yes² |
| **Demonstration** | `ace-tests` | `ace-api` and the whole workspace, including `ace-sim` | no — hosted only |

¹ `ace-macros` is a proc-macro crate. It is compiled for the host toolchain and
contributes nothing to the target binary.

² Builds for a bare-metal target, but see the caveat below.

Two things an integrator should notice:

- **`ace-server` does not include `ace-can`.** An ECU speaking UDS over CAN needs
  `ace-server` *and* `ace-can`; the server itself is transport-agnostic.
- **Nothing outside the demonstration subset depends on `ace-sim`.** Simulation
  infrastructure — virtual clock, seeded RNG, buses, fault injection — is never
  pulled into a production build. The abstractions those implement live in
  `ace-core`.

## Verifying a subset

Each subset is proved by building it in isolation, with default features off, for a
bare-metal target. `thumbv7em-none-eabihf` (Cortex-M4F) is used as the reference:

```sh
rustup target add thumbv7em-none-eabihf

cargo build -p ace-server  --no-default-features --target thumbv7em-none-eabihf
cargo build -p ace-client  --no-default-features --target thumbv7em-none-eabihf
cargo build -p ace-gateway --no-default-features --target thumbv7em-none-eabihf
```

A host build with `--no-default-features` is **not** sufficient evidence: the host
has `std` available, so a dependency that silently requires it still links. Only a
`*-none-*` target proves the claim.

That is not hypothetical. Until this was checked, `ace-doip` and `ace-gateway`
could not build for any bare-metal target: the workspace declared `derive_more`
with its default features, which enable `std`, and `derive_more` does not declare
`#![no_std]`. The root README's `no_std` claim was false for those two crates. It
now holds, and `derive_more` is declared `default-features = false` with consumers
opting into the derives they need.

## Feature unification hides defects

`cargo check --workspace` compiles every crate with features **unified across the
whole graph**. Because `ace-tests` depends on the `ace-api` facade, which enables
`ace-core/alloc`, a workspace-wide check silently builds the protocol crates in
their `alloc` configuration. Defects present only in the no-allocator
configuration do not appear.

Concretely: `ace_core::Vec` is `heapless::Vec` without `alloc` and
`alloc::vec::Vec` with it. Their `push` methods have different signatures —
`Result` versus `()` — so ignoring a `push` result is a warning in one
configuration and correct in the other. A workspace check reports zero warnings
while a per-crate build reports twelve.

Two consequences:

1. Per-crate, per-feature builds are the real gate. Phase 7's CI matrix exists for
   this reason.
2. `ace_core::Vec` presenting a different API per feature is itself a library
   defect. Phase 3 gives storage a uniform push contract.

## Known caveat

`ace-gateway` builds for a bare-metal target but emits 12 `unused Result`
warnings, every one a dropped `heapless::Vec::push` result:

```
crates/ace-gateway/src/ecu_node.rs:245  crates/ace-gateway/src/tester.rs:742
crates/ace-gateway/src/ecu_node.rs:335  crates/ace-gateway/src/tester.rs:747
crates/ace-gateway/src/gateway.rs:323   crates/ace-gateway/src/tester.rs:752
crates/ace-gateway/src/gateway.rs:707   crates/ace-gateway/src/tester.rs:757
crates/ace-gateway/src/gateway.rs:712   crates/ace-gateway/src/tester.rs:797
crates/ace-gateway/src/isotp_node.rs:265 crates/ace-gateway/src/tester.rs:802
```

These are deliberately left for Phase 3 rather than silenced. Each needs
individual judgement: some are guarded by a preceding capacity check and are
therefore safe, others may be genuine silent drops of protocol data. Blanket
`let _ =` would cement the unsafe ones and destroy the evidence, which is the
opposite of the Phase 3 goal.
