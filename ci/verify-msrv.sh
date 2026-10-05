#!/usr/bin/env sh
set -eu

# CI should install this toolchain before invoking the script. Override for a
# locally installed compatible patch release if needed.
JOBS="${CARGO_BUILD_JOBS:-2}"

for crate in \
    ace-core ace-macros ace-proto ace-can ace-sim ace-uds ace-doip ace-server ace-client ace-gateway ace-api
 do
    cargo msrv verify \
        --manifest-path "crates/$crate/Cargo.toml" \
        -- cargo check --all-targets -j "$JOBS"
done
