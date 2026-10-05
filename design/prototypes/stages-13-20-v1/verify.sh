#!/usr/bin/env bash
set -euo pipefail
candidate_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$candidate_repo_root"
output_root="${1:-$(mktemp -d /tmp/keystone-candidates.XXXXXX)}"
mkdir -p "$output_root"
for stage_id in 13 14 15 16 17 18 19 20; do
  cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
    check "$stage_id" \
    --stage-file "design/prototypes/stages-13-20-v1/stage-${stage_id}.ron" \
    --plan "design/prototypes/stages-13-20-v1/stage-${stage_id}.plan" \
    --seed 0 --seeds 100 --walk-to-goal \
    --require-initially-unreachable --reject-blocked \
    --output "$output_root/stage-${stage_id}"
done
