#!/usr/bin/env bash
set -euo pipefail

design_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$design_repo_root"

mapfile -t catalog_ids < <(sed -nE 's/.*id: ([0-9]+).*/\1/p' assets/stages/list.ron)

if [[ "${#catalog_ids[@]}" -ne 20 ]]; then
  echo "expected 20 catalog entries, found ${#catalog_ids[@]}" >&2
  exit 1
fi

stage_sim=(cargo run --quiet --manifest-path tools/stage_sim/Cargo.toml --)

for stage_id in $(seq 1 20); do
  catalog_index=$((stage_id - 1))
  if [[ "${catalog_ids[$catalog_index]}" != "$stage_id" ]]; then
    echo "catalog entry $catalog_index is ${catalog_ids[$catalog_index]}, expected $stage_id" >&2
    exit 1
  fi

  stage_path="assets/stages/stage-${stage_id}.ron"
  if [[ ! -f "$stage_path" ]]; then
    echo "missing product stage: $stage_path" >&2
    exit 1
  fi

  if ! rg -q "stage-${stage_id}\\.ron" src/resources/stage_catalog.rs; then
    echo "StageCatalog does not embed $stage_path" >&2
    exit 1
  fi

  "${stage_sim[@]}" analyze "$stage_id" --stages-dir assets/stages >/dev/null
done

for stage_id in 1 2 3 4 5 6 7 8; do
  if ! cmp -s "design/stages/stage-${stage_id}.ron" "assets/stages/stage-${stage_id}.ron"; then
    echo "product Stage $stage_id differs from the fixed design baseline" >&2
    exit 1
  fi
done

echo "product stage catalog: 20 entries parsed; fixed Stage 1-8 match the design baseline"
