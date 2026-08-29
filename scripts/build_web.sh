#!/usr/bin/env bash
set -euo pipefail

# Trunk expects this environment variable to be a boolean, while some shells
# expose NO_COLOR as `1`.
NO_COLOR=false trunk build --release
