#!/usr/bin/env python3
"""Repeat the full 1,800,000-frame release benchmark; never a CI speed gate."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--jobs", type=int, default=24)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
os.chdir(root)
env = dict(os.environ)
for key in ("DISPLAY", "WAYLAND_DISPLAY"):
    env.pop(key, None)
source = json.loads(Path("examples/headless/benchmark.json").read_text())
summary = []
for count, jobs in [(1, 1), (1000, args.jobs)]:
    source["seed_count"] = count
    scenario = Path(f"target/headless-benchmark-{count}.json")
    report = Path(f"target/headless-benchmark-{count}-report.json")
    scenario.write_text(json.dumps(source))
    started = time.monotonic()
    with Path(f"target/headless-benchmark-{count}.log").open("w") as log:
        subprocess.run(["target/release/keystone-cc", "--headless-verify", str(scenario),
                        "--report", str(report), "--jobs", str(jobs)], env=env,
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    wall = time.monotonic() - started
    result = json.loads(report.read_text())
    assert result["frames"] == count * 1800
    summary.append({"count": count, "jobs": jobs, "process_wall_seconds": wall,
                    **{key: result[key] for key in
                       ("elapsed_seconds", "frames", "frames_per_second")}})
Path("target/headless-benchmark-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
