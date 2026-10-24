# Headless stage verification (schema version 1)

Run from the repository root, after `cargo build --release`:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify examples/headless/stage-1.json --report target/stage-1-report.json

env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify examples/headless/stage-2.json --report target/stage-2-report.json

env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-measure-reach --report target/reach-report.json

python3 examples/headless/benchmark.py --jobs 24
```

No window, renderer, audio device, Steam client, egui, VPL, external control server,
settings file or save file is opened. The binary still compiles with the product's
normal dependencies. Player/tile sprites use placeholder image/atlas handles; their
transforms, colliders and physics are unchanged. A fixed 1800×1200 viewport with
scale 1 supplies the product's design resolution. The product canvas is always
30×20 cells (60 px per cell); RON `map_size` specifies the interior and margins.

Each world starts with Bevy's ordinary zero-delta initialization update, then
advances by `Duration::from_secs_f64(1.0 / 60.0)` per update without sleeping.
`Time<Fixed>` is not overridden. Frame 0 is the first timed update. Inputs are
injected before that update; the product reads them in `Update` as usual.

## Scenario JSON

Unknown fields are errors. Paths are relative to the process working directory.

| Field | Type / default | Meaning |
| --- | --- | --- |
| `stage_id` | integer 1–12 | Loads `assets/stages/stage-N.ron`; use exactly one stage selector |
| `ron_path` | string | Alternative RON path; uses the same product generator |
| `seed_start` | u64, required | First terrain seed |
| `seed_count` | positive integer, required | Consecutive seeds to run |
| `start_frame` | u64, 0 | Compile/start all programs and enable controls at this frame, like Run |
| `programs` | array, `[]` | Entries `{ "stone_index": 0, "language": "Rhai", "code": "move(\"down\");" }` |
| `inputs` | array, `[]` | Entries `{ "frame": 30, "key": "ArrowRight", "pressed": true }` |
| `seed_inputs` | object, `{}` | Decimal seed keys map to replacement input arrays, e.g. `"17": [...]` |
| `max_frames` | positive u64, required | Maximum timed updates; first goal or script failure stops early |
| `trajectory_every` | positive u64, 10 | Sampling interval; terminal frame is always included |

Stone indices are zero-based spawn order. Languages are `Rhai` and `Keystone`.
Missing programs compile as empty Rhai programs; duplicate or nonexistent stone
indices are errors. Product `ScriptExecutor` compilation and `StoneCapabilities`
restrictions apply. Rhai uses the same interpreter with a blocking per-command
handoff in this mode, so a 1ms UI timeout cannot change the emission frame under
CPU load. A headless-only 100,000-operation budget per step reports a script error
instead of hanging on a non-yielding Rhai loop. Normal interactive timeouts and
execution remain unchanged. The headless Rhai interpreter rejects `timestamp()`
(including dynamic calls) because it reads wall-clock time. Accepted keys: `ArrowLeft`, `ArrowRight`, `ArrowUp`, `KeyA`,
`KeyD`, `KeyW`, `Space`. Pressed keys stay held until a release; edges clear every
frame. Events sort by frame, retaining input array order within a frame.
`start_frame` and all input frames must be below `max_frames`.

Keystone's current VM implements `rand` with an internal, unseedable RNG. This
mode conservatively rejects the `rand` identifier (including in comments) with a
compile error. It does not silently produce nondeterministic verification results.
Supporting it requires an upstream VM seed interface. Normal gameplay is unchanged.
The unused random value produced by `tick_script_program` is not exposed by either
product executor. Tile decoration is also seeded: product moss tiles add duplicate colliders, so
leaving decoration random would change collision results. Visual asset handles are
excluded from hashes.

`--jobs N` (1–256, default 1) runs independent Bevy worlds on worker threads.
Reports sort by seed irrespective of worker completion order. Each world's schedules use single-thread execution; the headless process requests
one compute-pool thread to avoid nested ECS parallelism. Avian tree optimization
uses its unchanged algorithm inline rather than through the shared async pool.

## Report JSON

`schema_version`, `seeds`, `count`, `cleared`, `frames`, `elapsed_seconds`, and
`frames_per_second` comprise the report. Timing includes map generation, world
construction, execution, and teardown; excludes report serialization/file output.
The benchmark script also measures complete process wall time, and verifies that
all 1,800,000 frames ran (idle input avoids early goal exits).

Each seed contains `cleared`, nullable `goal_frame`, `frames`, `termination`
(`goal`, `max_frames`, `compile_error`, `script_error`), nullable `error`,
`terrain_ascii`, `terrain_hash`, `execution_hash`, `commands`, and `trajectory`.
A failed compile or script is a reportable simulation outcome; malformed scenario
or unreadable files cause exit 2. Successful report generation exits 0, even if a
seed did not clear.

ASCII is top-down; `*` solid, `#` boundary wall, `@` spawn, `S` stone, `G` goal,
`O` obstacle, `?` dynamic solid, `.` empty. Trajectory cells use bottom-left origin
on the full 30×20 canvas; world coordinates are pixels relative to its center.
Commands include zero-based `action_id`, `stone_index`, product debug command,
`issued_frame`, nullable `completed_frame`, and `result` (`completed`, `blocked`,
`pending`). Correlation IDs use existing product outcome messages without starting
the external-control server. Pending at termination does not mean completed.

Hashes use versioned FNV-1a 64-bit (regression fingerprint, not cryptographic).
Terrain hashes cover ASCII. Execution hashes cover the scenario, seed, terrain,
every frame's player/stone positions and velocities sorted by stone index,
obstacle lifetime and collider state sorted by position, and
all deterministic report fields including outcomes and sampled trajectory.
Timing and visual assets are excluded. Seeded obstacle lifetime uses the product's
existing duration distributions; dynamic solid toggles also use a seeded stream; normal play continues to use its unseeded RNG. Hashes are intended for the same build,
dependency lockfile and platform; no cross-architecture floating-point guarantee.

## Reach measurements

The report includes every trial plus maximum successful integer step/gap size in
cells and pixels, separately for standing/running and left/right, for interior
sizes (26,10), (28,10), (28,18). A zero-size control is included. The protocol uses
30 settling frames and a single jump from source coordinates 5.5, 6.0, 6.5, 6.75,
and 6.9 (ledge at 7). Standing trials first pre-position using ordinary input,
then stand for 30 frames; running trials approach from cell 3. Movement stops one
cell into the destination, and six stable landing frames are required. Trial limit
is 420 frames. Each trial records requested and actual takeoff positions. Tested steps are 0–5 and gaps 0–8 cells; all failures are retained.
`-1` means even the zero-size control failed. This is an empirical bound for the
documented inputs, not exhaustive route search. `hypothesis_equal_cell_reach`
compares all corresponding maxima across map sizes and requires valid controls.

The product currently assigns horizontal velocity immediately, so running does
not build acceleration. Both viewport and tile scale remain fixed across these
interior sizes. The result tests current product behavior, including collision
geometry and the phase difference between `Update` and the fixed physics clock.
