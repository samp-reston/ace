#!/usr/bin/env sh
set -eu

# Keep CI and local release verification on the same commands. Do not point
# CARGO_TARGET_DIR at /tmp: embedded matrix builds are large and the host may
# use tmpfs there.
JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_BUILD_JOBS="$JOBS"
TARGET="thumbv7em-none-eabihf"

cargo fmt --all --check
cargo hack check --workspace --each-feature --locked
cargo check --workspace --all-targets -j "$JOBS"
cargo test --workspace --no-fail-fast -j "$JOBS"

# Explicit feature configurations that workspace feature unification can hide.
cargo test -p ace-core --no-default-features -j "$JOBS"
cargo test -p ace-core --features alloc -j "$JOBS"
cargo test -p ace-tests --no-default-features -j "$JOBS"

# Every production protocol/simulation crate must build for a real embedded target
# without default features. ace-tests, ace-api, and ace-transport are host-side
# integration/facade crates and are intentionally outside this target matrix.
for crate in \
    ace-core ace-proto ace-uds ace-can ace-doip ace-server ace-client ace-gateway ace-sim
 do
    cargo build -p "$crate" --no-default-features --target "$TARGET" -j "$JOBS"
done
