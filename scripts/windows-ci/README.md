# Windows CI smoke contract

`invoke-smoke.ps1` launches a built `keystone-cc.exe` with `--ci-smoke` and an
explicit report path. It sets `KEYSTONE_CI_SAVE_DIR` to an isolated directory,
enforces a 1–300 second timeout, stops the process tree on timeout, and accepts
success only when the report says the boot assets are ready and the app entered
`SelectStage`. It does not assert that a frame was rendered.

The application requires both `--ci-smoke-report <path>` and
`KEYSTONE_CI_SAVE_DIR` for this opt-in path. Normal launches keep their usual
save directory.

`run-smoke-job.ps1` is an optional one-shot wrapper for the existing A1X queue
worker. It makes no network calls and reads no secrets. The worker supplies
`SASARA_JOB_PAYLOAD` as a staging directory and `SASARA_JOB_OUTPUT` as an output
directory. The staging directory must contain `invoke-smoke.ps1`, the artifact
under `artifact/`, and this `smoke-job.json`:

```json
// smoke-job.json
{
  "artifact_dir": "artifact",
  "executable_path": "artifact\\keystone-cc.exe",
  "timeout_seconds": 120
}
```

`executable_path` and `timeout_seconds` are optional. The wrapper writes its
machine-readable result to `SASARA_JOB_OUTPUT/result.json`; its sibling files
contain the smoke report, log, and isolated save data.

The owner-only A1X bridge is invoked manually after a successful hosted run; it
does not run from a GitHub trigger or register a runner. Supply the local queue
CLI explicitly (or via your own wrapper):

```bash
python3 scripts/windows-ci/a1x-smoke-bridge.py --repo OWNER/REPOSITORY \
  --run-id RUN_ID --expected-sha COMMIT_SHA \
  --queue-script /path/to/windows-jobs.py --output /safe/output/result.zip
```

It verifies the exact workflow, repository, successful conclusion, artifact
metadata SHA, active A1X worker, and queue result archive hash before reporting
success. The `--output` path is the only place it copies the returned archive.
