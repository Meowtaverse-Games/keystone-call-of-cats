#!/usr/bin/env bash
set -euo pipefail

design_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$design_repo_root"

stage_sim=(cargo run --quiet --manifest-path tools/stage_sim/Cargo.toml --)

for stage_id in 5 6 7 8; do
  "${stage_sim[@]}" analyze "$stage_id" \
    --stages-dir design/stages
  "${stage_sim[@]}" run "$stage_id" \
    --stages-dir design/stages \
    --stone-script "design/solutions/stage-${stage_id}-stone-0.ks" \
    --player-plan "design/solutions/stage-${stage_id}-player.plan" \
    --max-rounds 100
done
