#!/bin/bash
set -e

# Mimic the environment variable from the workflow
export CARGO_TERM_COLOR=always

echo "Starting MacOS build..."

# Run the build command
cargo build --release

# Copy the steam library to the output directory
find target/release/build -name "libsteam_api.dylib" -exec cp {} target/release/ \;

echo "Build complete."
echo "Artifact location: target/release/keystone-cc"
