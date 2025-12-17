#!/bin/bash
set -e

# Mimic the environment variable from the workflow
export CARGO_TERM_COLOR=always

echo "Starting MacOS build..."

# Run the build command
cargo build --release

echo "Build complete."
echo "Artifact location: target/release/keystone-cc"
