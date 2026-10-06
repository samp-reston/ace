# Coding standards

Judgement-call standards for reviewing changes to this workspace. Mechanical
rules (formatting, lints, feature/target/MSRV/semver) are enforced by CI via
`ci/verify-*.sh`; this file covers what no linter can — the design invariants a
reviewer must check by reading the diff. Terms and fixed values are in
[`GLOSSARY.md`](GLOSSARY.md).

## 1. `ace` is a suite of libraries, not a product

`ace` is a suite of libraries for *building* a diagnostic stack — never a stack,
daemon, simulator, or reference oracle itself. The deterministic vehicle
simulation is a **demonstration** of the suite's sufficiency, delivered through
the `ace-tests` crate.

- Protocol behaviour that lives in a consumer or test harness instead of in a
  library crate is a **library gap**, not a feature of the demonstration.
- Judge changes by library obligations: composability, layer independence,
  subset usability, extension traits, API/resource contracts, docs.
- `ace-tests` composes the individual crates directly (not the `ace-api`
  facade), so it also proves the suite composes for an embedded target.

## 2. Units stand alone and compose upward

Every unit (a server, client, node, gateway) must be usable standalone and be
composition-ready:

- Declared identity/addresses; **no globals or singletons**.
- Injected clock and transport — no wall-clock reads, no hidden I/O.
- Explicit ownership filtering: a node acts only on frames and link/connection
  events addressed to its own identity. Broadcasting events to every node on a
  segment is fine only if every node self-filters by endpoint identity.
- Bounded per-instance resources (const-generic capacities).
- Composition layers depend on units, never the reverse. Do not introduce a
  mandatory parent/container type (Vehicle, System, Cluster).

## 3. `no_std` / bare-metal is a contract, not an aspiration

- Every production protocol/simulation crate must build for
  `thumbv7em-none-eabihf` with `--no-default-features`. `alloc` is opt-in; the
  `ace-api` facade is the only alloc-by-default entry point.
- Verify per-crate for a real embedded target. A clean host
  `--no-default-features` build or a workspace-unified `cargo check` is **not**
  proof — the host has `std` for a dependency to reach, and feature unification
  enables `alloc`/`std` across the graph and hides per-crate defects.

## 4. Errors are explicit; fail closed

- No silent drops on bounded storage. A `push` that cannot fit is a propagated,
  typed error — never `let _ = v.push(...)` on a path a caller depends on.
- Security-relevant failures fail **closed**: if an attempt/lockout cannot be
  recorded, deny rather than proceed.
- No `panic!`/`unwrap`/`expect`/`unreachable!` on untrusted wire input in
  production paths. `debug_assert!` for caller-contract invariants is fine.

## 5. ISO 13400 / 14229 terminology

- "Entity" is the generic DoIP peer. `EntityStatusResponse.node_type`
  distinguishes Gateway Entities from Node Entities. The post-RoutingActivation
  VehicleIdentificationRequest applies **only** to Gateway Entities. Name types
  `EntityId`/`EntityLink`, not `Gateway*`.
- NRCs ISO 14229-1 requires a server to withhold on a functionally addressed
  request are suppressed; a server-specific failure (the request *is* ours and
  processing failed) is answered regardless of addressing or the
  suppress-positive bit.

## 6. Tests prove behaviour, not just shape

- When two implementations are indistinguishable on the wire in the cases that
  matter (e.g. "response withheld" vs "request not served" both look like
  silence), assert on **side-effect state** (a session/security change), not on
  the response.
- Pair a fix with a **control** test: removing the fix fails, restoring passes.
- Property tests assert **identity** (`decode(encode(v)) == v`), not mere
  idempotence, and must not silently skip the body when a generator misfires.
- No `#[ignore]` without a written justification.

## 7. Release discipline

- Public API changes gate the version. `cargo-semver-checks` runs at tag time
  against the crates.io baseline; proc-macro-only crates (`ace-macros`) are
  excluded (no library target to inspect).
- MSRV is validated against the real dependency graph, not the edition floor.
- The canonical local/CI check is `sh ci/verify-matrix.sh`. Keep local
  verification and CI on the same scripts.
