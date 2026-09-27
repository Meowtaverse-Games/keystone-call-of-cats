# External stone control

Launch with `--external-control` and set `KEYSTONE_EXTERNAL_CONTROL_TOKEN` to a private random value. The server binds only to `127.0.0.1:38473` (override with `--external-control-port`). It rejects requests carrying an `Origin` header.

`GET /v1/state` returns the current generation, stone indices, world-pixel positions, capabilities and budgets. Start a session with `POST /v1/session/start`, then submit one command to `POST /v1/stones/{index}/commands`. Commands are `move`, `dig`, `place` with a cardinal `direction`, or `sleep` with finite `duration` up to 60 seconds. Poll `GET /v1/actions/{id}` until its status becomes `complete` or `rejected`; `202 Accepted` only means the game accepted the request into its bounded queue.

Every request needs `Authorization: Bearer <token>`. A reset, stage change, stop, or game exit invalidates pending work. This API is an input source and does not add abilities or execute arbitrary code.
