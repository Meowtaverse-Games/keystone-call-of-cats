#!/usr/bin/env python3
"""Measure the replay-backed checkpoint oracle; never enforce a CI speed threshold.

Run after `cargo build --release --locked`. All generated files stay under target.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', default='target/release/keystone-cc')
parser.add_argument('--output', default='target/snapshot-benchmark')
args = parser.parse_args()
os.chdir(Path(__file__).resolve().parents[2])
output = Path(args.output)
output.mkdir(parents=True, exist_ok=True)
env = dict(os.environ)
for key in ('DISPLAY', 'WAYLAND_DISPLAY'):
    env.pop(key, None)
reports = []
for label, source, language, code in [
    ('no-code', 'snapshot-terrain', None, None),
    ('rhai-dig', 'dig', 'Rhai', 'sleep(0.2); dig("down"); sleep(0.2); move("right");'),
    ('keystone-dig', 'dig', 'Keystone', 'sleep 0.2\ndig down\nsleep 0.2\nmove right'),
    ('rhai-place', 'snapshot-place', 'Rhai', 'place("up"); move("right"); move("left");'),
    ('keystone-place', 'snapshot-place', 'Keystone', 'place up\nmove right\nmove left'),
]:
    scenario = json.loads(Path(f'examples/headless/{source}.json').read_text())
    scenario.update(max_frames=1200, trajectory_every=1)
    if language:
        scenario['programs'] = [dict(stone_index=0, language=language, code=code)]
    scenario_path = output / f'{label}.json'
    report_path = output / f'{label}-report.json'
    scenario_path.write_text(json.dumps(scenario, indent=2) + '\n')
    with (output / f'{label}.log').open('w') as log:
        subprocess.run([args.binary, '--headless-measure-snapshot', str(scenario_path),
                        '--report', str(report_path)], env=env, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    report = json.loads(report_path.read_text())
    if report['profile'] != 'release':
        raise SystemExit('Benchmark requires a release binary')
    report['label'] = label
    reports.append(report)
    print(label, flush=True)
metadata = dict(platform=platform.platform(), cpu_count=os.cpu_count(),
                rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                protocol='20 samples per depth, one warm-up, alternating restore/replay order; no speed gate',
                reports=reports)
if Path('/proc/cpuinfo').exists():
    metadata['cpu'] = next((line.split(':', 1)[1].strip() for line in
                           Path('/proc/cpuinfo').read_text().splitlines()
                           if line.startswith('model name')), 'unknown')
(output / 'summary.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(output / 'summary.json')
