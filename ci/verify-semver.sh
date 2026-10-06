#!/usr/bin/env sh
set -eu

# ace-macros is a proc-macro-only crate and cargo-semver-checks has no library
# target to inspect for it; the feature/target and MSRV matrices still cover it.
# ace-tests and ace-transport are publish=false. The remaining workspace crates
# are the public library/facade packages and have crates.io baselines.
for crate in \
    ace-core ace-proto ace-can ace-sim ace-uds ace-doip ace-server ace-client ace-gateway ace-api
 do
    cargo semver-checks --package "$crate"
done
