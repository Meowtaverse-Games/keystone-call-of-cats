#!/bin/bash
SCRIPT_DIR=$(cd $(dirname "$0") && pwd)
PROJECT_ROOT=$(dirname "$SCRIPT_DIR")
cargo run --manifest-path "$PROJECT_ROOT/tools/ftl_sheet_exporter/Cargo.toml" -- "$@"
