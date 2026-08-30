#!/usr/bin/env bash
set -euo pipefail

# Trunk expects this environment variable to be a boolean, while some shells
# expose NO_COLOR as `1`. Use an isolated target directory so native and web
# builds do not contend for Cargo's build lock. The dedicated Cargo profile
# avoids desktop release LTO, which is disproportionately expensive for Wasm.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/web}" NO_COLOR=false trunk build --cargo-profile web-release --locked
