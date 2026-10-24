#!/usr/bin/env python3
"""Run the six agreed viewport scales and retain raw trials plus comparisons."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--binary", default="target/release/keystone-cc")
parser.add_argument("--output", default="target/headless-reach")
parser.add_argument("--jobs", type=int, default=6)
args = parser.parse_args()
os.chdir(Path(__file__).resolve().parents[2])
output = Path(args.output)
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ)
for key in ("DISPLAY", "WAYLAND_DISPLAY"):
    env.pop(key, None)
scales = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0]


def measure(scale):
    path = output / f"reach-{scale}.json"
    with (output / f"reach-{scale}.log").open("w") as log:
        subprocess.run([args.binary, "--headless-measure-reach", "--viewport-scale",
                        str(scale), "--report", str(path)], env=env, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    report = json.loads(path.read_text())
    # Raw trials stay in the individual reports; the committed summary retains
    # every size/direction/approach maximum and all fixed-input observations.
    for measurement in report["measurements"]:
        measurement.pop("trials")
    return report


with ThreadPoolExecutor(max_workers=args.jobs) as executor:
    reports = list(executor.map(measure, scales))
baseline = reports[2]
equal = all(
    row["max_step_cells"] >= 0 and row["max_gap_cells"] >= 0
    and (row["max_step_cells"], row["max_gap_cells"])
    == (base["max_step_cells"], base["max_gap_cells"])
    for report in reports
    for row, base in zip(report["measurements"], baseline["measurements"])
)
summary = {"schema_version": 1,
           "hypothesis_equal_cell_reach_across_scales": equal,
           "reports": reports}
(output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps({"hypothesis_equal_cell_reach_across_scales": equal,
                  "summary": str(output / "summary.json")}))
