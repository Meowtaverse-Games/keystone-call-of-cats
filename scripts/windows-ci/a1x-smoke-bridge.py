#!/usr/bin/env python3
"""Owner-invoked bridge from a successful Windows artifact to the A1X queue."""
import argparse, hashlib, json, os, shutil, subprocess, sys, tempfile, time, zipfile
from pathlib import Path

WORKFLOW = '.github/workflows/windows_verify.yml'

def call(*args):
    return subprocess.run(args, check=True, text=True, capture_output=True)

def gh_json(*args): return json.loads(call('gh', *args).stdout)

def ensure_run(repo, run_id, expected_sha):
    run = gh_json('api', f'repos/{repo}/actions/runs/{run_id}')
    if run.get('repository', {}).get('full_name') != repo: raise ValueError('run repository mismatch')
    if run.get('path') != WORKFLOW: raise ValueError('run is not Windows verification workflow')
    if run.get('conclusion') != 'success': raise ValueError('run did not succeed')
    if run.get('head_sha') != expected_sha: raise ValueError('run SHA does not match --expected-sha')
    return run

def queue_json(queue, *args): return json.loads(call(sys.executable, str(queue), *args).stdout)

def require_worker(status):
    now = time.time(); workers = status.get('workers', [])
    if not any(w.get('device') == 'a1x' and now - w.get('seen', 0) < 120 for w in workers):
        raise ValueError('A1X worker is offline or stale')
    if any(j.get('device') == 'a1x' and j.get('state') in ('queued','running') for j in status.get('jobs', [])):
        raise ValueError('A1X already has an active job')

def validate_download(root):
    names = set(); total = 0
    for path in root.rglob('*'):
        rel = path.relative_to(root)
        if path.is_symlink() or any(part.lower() in ('.git','.ssh','.codex','.env') for part in rel.parts):
            raise ValueError(f'unsafe artifact entry: {rel}')
        if path.is_file():
            key = str(rel).casefold()
            if key in names: raise ValueError(f'case collision: {rel}')
            names.add(key); total += path.stat().st_size
            if total > 512 * 1024 * 1024: raise ValueError('artifact exceeds A1X queue limit')

def stage_artifact(repo, run_id, sha, staging):
    artifact_name = f'windows-verification-{sha}'
    artifacts = gh_json('api', f'repos/{repo}/actions/runs/{run_id}/artifacts').get('artifacts', [])
    if not any(a.get('name') == artifact_name and not a.get('expired') for a in artifacts):
        raise ValueError('expected Windows verification artifact was not found')
    downloaded = staging / 'download'; downloaded.mkdir()
    call('gh', 'run', 'download', str(run_id), '-n', artifact_name, '-D', str(downloaded), '-R', repo)
    validate_download(downloaded)
    binary = next(downloaded.rglob('keystone-cc.exe'), None)
    if binary is None: raise ValueError('artifact contains no keystone-cc.exe')
    artifact = staging / 'payload' / 'artifact'; artifact.mkdir(parents=True)
    shutil.copy2(binary, artifact / 'keystone-cc.exe')
    assets = next((p for p in downloaded.rglob('assets') if p.is_dir()), None)
    if assets: shutil.copytree(assets, artifact / 'assets')
    return artifact

def verify_result(job, output):
    directory = Path(job['directory']); report = json.loads((directory/'result.json').read_text())
    archive = directory/'result.zip'; digest = hashlib.file_digest(archive.open('rb'),'sha256').hexdigest()
    if report.get('id') != job.get('id') or report.get('state') != 'succeeded' or report.get('sha256') != digest:
        raise ValueError('queue result identity or archive hash mismatch')
    if output:
        output.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(archive, output)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run-id', required=True); p.add_argument('--expected-sha', required=True)
    p.add_argument('--repo', required=True); p.add_argument('--queue-script', required=True, type=Path)
    p.add_argument('--output', type=Path); p.add_argument('--wait-seconds', type=int, default=240)
    a=p.parse_args()
    if not a.queue_script.is_file(): raise ValueError('queue script not found')
    ensure_run(a.repo, a.run_id, a.expected_sha)
    require_worker(queue_json(a.queue_script, 'status'))
    with tempfile.TemporaryDirectory(prefix='keystone-a1x-') as temp:
        root=Path(temp); artifact=stage_artifact(a.repo,a.run_id,a.expected_sha,root)
        payload=root/'payload'; script=Path(__file__).with_name('run-smoke-job.ps1')
        shutil.copy2(Path(__file__).with_name('invoke-smoke.ps1'),payload/'invoke-smoke.ps1')
        (payload/'smoke-job.json').write_text(json.dumps({'artifact_dir':'artifact','executable_path':'artifact/keystone-cc.exe','timeout_seconds':180}))
        job=queue_json(a.queue_script,'submit','--script',str(script),'--payload',str(payload),'--device','a1x','--timeout','180','--title',f'keystone smoke {a.expected_sha}')
        deadline=time.time()+a.wait_seconds
        while time.time()<deadline:
            current=queue_json(a.queue_script,'show',job['id'])
            if current.get('state') in ('succeeded','failed','timed_out'):
                if current['state'] != 'succeeded': raise RuntimeError(f"A1X job {current['state']}")
                verify_result(current, a.output)
                print(json.dumps(current)); return
            time.sleep(5)
        current=queue_json(a.queue_script,'show',job['id'])
        if current.get('state') == 'queued':
            try: queue_json(a.queue_script,'cancel',job['id'])
            except subprocess.CalledProcessError: pass
        raise TimeoutError(f"A1X job remains {current.get('state')}; it was not reported successful")
if __name__=='__main__':
    try: main()
    except (ValueError, RuntimeError, TimeoutError, subprocess.CalledProcessError) as e:
        print(str(e),file=sys.stderr); sys.exit(1)
