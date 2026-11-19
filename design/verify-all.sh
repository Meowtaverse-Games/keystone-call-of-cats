#!/usr/bin/env bash
set -euo pipefail

design_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$design_repo_root"

./design/verify-stages-01-04.sh
./design/verify-stages-05-08.sh
./design/verify-stages-09-12.sh
./design/verify-stages-13-16.sh
./design/verify-stages-17-20.sh
