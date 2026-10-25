#!/usr/bin/env python3
"""Recheck frozen motion hashes, stage examples, and the complete reach summary."""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', default='target/release/keystone-cc')
parser.add_argument('--output', default='target/snapshot-baselines')
args = parser.parse_args()
os.chdir(Path(__file__).resolve().parents[2])
output = Path(args.output)
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ)
for key in ('DISPLAY', 'WAYLAND_DISPLAY'):
    env.pop(key, None)
entries = json.loads(Path('examples/headless/motion-baselines.json').read_text())['scenarios']
entries += [dict(scenario=f'examples/headless/stage-{stage}.json', trajectory_hash=value)
            for stage, value in [(1, 'fnv1a64-v1:8f5d6c37b6efc4fe'), (2, 'fnv1a64-v1:eb1f870862eed1bc')]]
for entry in entries:
    label = Path(entry['scenario']).stem
    path = output / f'{label}.json'
    with (output / f'{label}.log').open('w') as log:
        subprocess.run([args.binary, '--headless-verify', entry['scenario'], '--report', str(path)],
                       env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    report = json.loads(path.read_text())
    for seed in report['seeds']:
        assert seed['trajectory_hash'] == entry['trajectory_hash'], (label, seed['seed'], seed['trajectory_hash'])
    print(label, 'OK', flush=True)
subprocess.run(['python3', 'examples/headless/measure-scales.py', '--binary', args.binary,
                '--output', str(output / 'reach')], env=env, check=True)
actual = json.loads((output / 'reach' / 'summary.json').read_text())
expected = json.loads(Path('examples/headless/reach-scale-summary.json').read_text())
assert actual == expected, 'Reach summary changed'
print('All motion/stage hashes and all reach summary values unchanged')
