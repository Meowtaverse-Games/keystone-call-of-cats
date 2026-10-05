#!/usr/bin/env bash
set -euo pipefail

design_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$design_repo_root"

stage_sim=(cargo run --quiet --manifest-path tools/stage_sim/Cargo.toml --)

for stage_id in 13 14 15 16; do
  case "$stage_id" in
    13) place_limit=1 ;;
    14) place_limit=7 ;;
    15) place_limit=6 ;;
    16) place_limit=4 ;;
  esac

  "${stage_sim[@]}" analyze "$stage_id" \
    --stages-dir design/stages
  "${stage_sim[@]}" simulate "$stage_id" \
    --stages-dir design/stages \
    --place-limit "$place_limit" \
    --plan "design/solutions/stage-${stage_id}-player.plan"
done
