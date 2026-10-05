#!/usr/bin/env sh
set -eu

# ace-tests and ace-transport are publish=false. The remaining workspace crates
# are the public library/facade packages and have crates.io baselines.
for crate in \
    ace-core ace-macros ace-proto ace-can ace-sim ace-uds ace-doip ace-server ace-client ace-gateway ace-api
 do
    cargo semver-checks --package "$crate"
done
