#!/usr/bin/env bash
set -euo pipefail

design_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$design_repo_root"

stage_sim=(cargo run --quiet --manifest-path tools/stage_sim/Cargo.toml --)

"${stage_sim[@]}" simulate 1 \
  --stages-dir design/stages \
  --plan design/solutions/stage-1-player.plan

for stage_id in 2 3 4; do
  "${stage_sim[@]}" analyze "$stage_id" \
    --stages-dir design/stages
  "${stage_sim[@]}" run "$stage_id" \
    --stages-dir design/stages \
    --stone-script "design/solutions/stage-${stage_id}-stone-0.ks" \
    --player-plan "design/solutions/stage-${stage_id}-player.plan" \
    --max-rounds 50
done
