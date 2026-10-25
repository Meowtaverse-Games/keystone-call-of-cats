#!/usr/bin/env python3
"""Repeat the release agent benchmark; timing is evidence, never a CI gate."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', default='target/release/keystone-cc')
parser.add_argument('--jobs', type=int, default=24)
parser.add_argument('--seeds', type=int, default=1000)
parser.add_argument('--attempts', type=int, default=8)
parser.add_argument('--max-frames', type=int, default=1800)
parser.add_argument('--output', type=Path, default=Path('target/agent-benchmark'))
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
scenario = args.output / 'scenario.json'
scenario.write_text(json.dumps({
    'ron_path': 'examples/headless/fixtures/stage-1.ron',
    'seed_start': 0, 'seed_count': args.seeds, 'max_frames': args.max_frames,
}, indent=2) + '\n')
report_path = args.output / 'report.json'
env = os.environ.copy()
env.pop('DISPLAY', None)
env.pop('WAYLAND_DISPLAY', None)
command = [args.binary, '--headless-agent', str(scenario), '--report', str(report_path),
           '--jobs', str(args.jobs), '--agent-attempts', str(args.attempts)]
start = time.monotonic()
subprocess.run(command, env=env, check=True)
wall = time.monotonic() - start
report = json.loads(report_path.read_text())
assert report['count'] == args.seeds
assert len({s['seed'] for s in report['seeds']}) == args.seeds
summary = {key: report[key] for key in (
    'condition', 'count', 'cleared', 'unsolved', 'errors', 'clear_rate',
    'mean_attempts', 'mean_frames', 'elapsed_seconds', 'reach_measurement_seconds',
)}
summary.update(command=command, profile='release (build separately with cargo build --release)',
               process_wall_seconds=wall, target_seconds=300, within_target=wall <= 300,
               unique_terrain_hashes=len({s['terrain_hash'] for s in report['seeds']}),
               max_frames=args.max_frames, max_attempts=args.attempts,
               total_attempt_frames=sum(s['frames'] for s in report['seeds']),
               total_observation_frames=sum(s['observation_frames'] for s in report['seeds']))
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
