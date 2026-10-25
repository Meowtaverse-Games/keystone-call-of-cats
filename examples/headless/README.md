# Headless stage verification and windowed replay (report schema version 2)

Run from the repository root, after `cargo build --release`:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify examples/headless/stage-1.json --report target/stage-1-report.json

env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify examples/headless/stage-2.json --report target/stage-2-report.json

env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-measure-reach --report target/reach-report.json

python3 examples/headless/benchmark.py --jobs 24
python3 examples/headless/measure-scales.py --jobs 6
```

No window, renderer, audio device, Steam client, egui, VPL, external control server,
settings file or save file is opened. The binary still compiles with the product's
normal dependencies. Player/tile sprites use placeholder image/atlas handles; their
transforms, colliders and physics are unchanged. A 1800×1200 design viewport with configurable scale (default 1) supplies the product's design resolution. The product canvas is always
30×20 cells (60 px per cell); RON `map_size` specifies the interior and margins.

Each world starts with Bevy's ordinary zero-delta initialization update, then
advances by `Duration::from_secs_f64(1.0 / 60.0)` per update without sleeping.
`Time<Fixed>` is not overridden. Frame 0 is the first timed update. Inputs are
injected before that update; the product reads them in `Update` as usual.

## Windowed scenario replay

```sh
target/release/keystone-cc --replay examples/headless/stage-1.json --seed 17 \
  --report target/stage-1-replay.json
target/release/keystone-cc --replay examples/headless/stage-2.json --seed 17 \
  --replay-speed 0.5 --report target/stage-2-replay.json
```

Replay opens the product window, renderer, player animation, audio, egui stage UI
and VPL. It enters the selected seeded stage directly after loading real assets.
Keyboard/mouse input to the simulation/editor is locked; only the scenario's input
schedule drives the player. Close the window normally to exit. At goal, maximum
frames or script error, physics and gameplay stop; the result and frame count stay
on screen and are logged. The optional report is written at that point. Closing
before completion does not write a completed report.

Simulation updates use the same manual 1/60-second duration and default fixed clock
as headless. Updates are paced by a monotonic clock to at most 60 per second,
independent of monitor refresh; rendering too slowly slows playback rather than
skipping simulation frames. `--replay-speed` (0.01–100, default 1) changes only this
real-time pacing, never simulation delta. Asset loading uses zero simulation time;
the first stage update is the same zero-delta bootstrap as headless.

The replay window requests native maximization on startup: the OS fits it to the
launch monitor's work area, accounting for DPI, taskbars and window decorations.
Its restored size is 640×360 logical points; users can restore, resize, maximize,
or close it normally. No monitor dimensions are used to change simulation settings.
The result overlay is measured from its text each frame and centered in the region
to the right of the reported editor-panel width. Short messages shrink to their
content; long messages wrap within that region. The camera reserves the measured
overlay height plus a gap above the canvas, so the status cannot cover its top row.
Panel-width, status-text and window-size changes update both layouts each frame.
**Physics viewport scale is fixed to the scenario's
`viewport_scale` (default 1), including its explicit `scale_changes`.** The camera
alone zooms out and shifts its view so the entire 30×20-cell canvas fits to the
right of the opaque editor panel, with a small margin. The stage root, physical
coordinates, `ScaledViewport`, and simulation remain unchanged. Camera projection
and translation are updated after egui finishes layout and before camera/transform
updates, using the reported panel width and screen dimensions in egui points
(so UI/DPI scale is accounted for). Window resizing or a changed reported panel
width recomputes the framing, including after playback stops. Manual input remains
locked, so dragging the panel is disabled in replay; any layout-driven width
change still triggers the same fit. This display zoom is separate from physical
viewport scale and does not affect hashes.
Use identical scenario scale settings when comparing reports. `--seed` selects one
seed, including its `seed_inputs` override, independently of the batch range.
`--viewport-scale` is only a headless/reach override, not a replay option.

Replay does not install the settings or progression/save plugins, Steam, or the
external-control server. It uses default settings, empty script storage, and a
no-op storage backend whose loads always return `None` and saves never persist.
It never opens user save/config files. Normal launches retain their existing setup.
The presentation systems are shared with normal play; simulation, program startup,
input injection and observation share the headless driver. Both verification modes
use deterministic Rhai synchronization and the same script restrictions below.

Compare `seeds[0].trajectory_hash`, not `execution_hash`, when a batch run and a
single-seed replay use different batch metadata. The hash includes every frame,
not just the trajectory samples. The windowless replay-schedule test matches both example trajectories, commands and
goal frames. Claude verified native Windows release/default-features at `b6dbaed`:
both headless and windowed replay matched Linux for seed 17, without panics or
schedule cycles. The stage appeared to the right of the editor panel.

| Scenario | Windows headless / replay and Linux trajectory hash | Goal frame |
| --- | --- | --- |
| Stage 1 | `fnv1a64-v1:8f5d6c37b6efc4fe` | 787 |
| Stage 2 | `fnv1a64-v1:eb1f870862eed1bc` | 1087 |

That check also found the old 1800×1200 logical window exceeded a 1920×1200 monitor
at 175% DPI, and the result text wrapped word by word. Native maximization and an
explicit result width addressed those issues. Claude rechecked `95148e8` on
Windows at 1920×1200 / 175%: maximization worked,
the entire stage was visible, and both stages' headless/replay hashes still matched
Linux. The status was partly hidden behind the panel because it was centered over
the whole window. The measured, panel-relative overlay and reserved camera space
now address that issue. **Claude will recheck this status layout on Windows.**
Windowless tests measure the real egui text rectangles at representative 175%,
100% and 125% logical sizes and panel widths of 25%, 40% and 50%, checking that
they stay beside the panel and above the canvas, and short messages shrink.

WSLg cannot run replay on this machine: debug panics in winit 0.30.13
Wayland keyboard initialization (`attempt to add with overflow`); Claude also
confirmed release exits with Wayland `Broken pipe` after creating a window, while
X11 cannot start because `libxkbcommon-x11` is missing. No windows were opened for
this revision, and no workaround or dependency installation was attempted.
`replay-validation.json` retains the previous debug attempts, not successful
replay reports. The asset-root lookup observed in those attempts was corrected to
the repository `assets` directory.

After generating both reports, compare the deterministic observations (wall time
and FPS intentionally differ):

```python
import json
from pathlib import Path
for stage in (1, 2):
    offline = json.loads(Path(f"target/stage-{stage}-report.json").read_text())["seeds"][0]
    replay = json.loads(Path(f"target/stage-{stage}-replay.json").read_text())["seeds"][0]
    for field in ("trajectory_hash", "trajectory", "commands", "goal_frame"):
        assert offline[field] == replay[field], (stage, field)
```

## Scenario JSON

Unknown fields are errors. Paths are relative to the process working directory.

| Field | Type / default | Meaning |
| --- | --- | --- |
| `viewport_scale` | positive finite number, 1.0 | Initial `ScaledViewport.scale`; also applied to the stage root |
| `scale_changes` | array, `[]` | Entries `{ "frame": 105, "scale": 0.75 }`; simulate viewport resize before that update |
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
CPU load. Both resume notification and command reception rendezvous with the VM
worker; a worker that has not yet started waiting cannot lose its resume signal. A verification-only 100,000-operation budget per step reports a script error
instead of hanging on a non-yielding Rhai loop. Normal interactive timeouts and
execution remain unchanged. The verification Rhai interpreter rejects `timestamp()`
(including dynamic calls) because it reads wall-clock time. Accepted keys: `ArrowLeft`, `ArrowRight`, `ArrowUp`, `KeyA`,
`KeyD`, `KeyW`, `Space`. Pressed keys stay held until a release; edges clear every
frame. Events sort by frame, retaining input array order within a frame.
`start_frame`, input frames, and scale-change frames must be below `max_frames`.
Duplicate scale-change frames are rejected. `--viewport-scale S` overrides the initial
scenario scale, and selects the scale for reach measurements (default 1.0).
The root transform uses the same viewport operation as graphical play, centered
at the world origin; the product itself still updates gravity, speed, and jump
velocity. Resizing does not reset physics: the shared product gameplay system
converts carried world-space velocities and stone travel distances by the new/old
root-scale ratio, including while player controls are disabled.

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
For replay, timing spans timed playback only (including real-time pacing),
excluding asset loading; it is not a headless throughput benchmark.
The benchmark script also measures complete process wall time, and verifies that
all 1,800,000 frames ran (idle input avoids early goal exits).

Each seed contains `cleared`, nullable `goal_frame`, `frames`, `termination`
(`goal`, `max_frames`, `compile_error`, `script_error`, `observation_error`), nullable `error`,
`terrain_ascii`, `terrain_hash`, `execution_hash`, `trajectory_hash`, `commands`, and `trajectory`.
A failed compile or script is a reportable simulation outcome; malformed scenario
or unreadable files cause exit 2. Successful report generation exits 0, even if a
seed did not clear.

ASCII is top-down; `*` solid, `#` boundary wall, `@` spawn, `S` stone, `G` goal,
`O` obstacle, `?` dynamic solid, `.` empty. Trajectory cells use bottom-left origin
on the full 30×20 canvas; world coordinates are pixels relative to its center. Cells are obtained using the
actual stage-root inverse transform, so viewport scaling is removed from cell
coordinates. World px values retain scaling.
Schema 2 adds `stones`, `placed_tiles`, and `dug_tiles` to every trajectory sample.
Each sample retains the player's `frame`, `cell: [x,y]`, and `world_px: [x,y]`.
Each stone has `stone_index`, `cell`, and `world_px`. Placed/dug tiles have `cell`
and `world_px`; these arrays are sorted by position and deduplicated. `placed_tiles`
contains only currently present platforms: entities marked `DugTile` are excluded,
even if they retain `PlacedTile`. Such entities appear only in `dug_tiles` until
restored. `observation_error` is a replay-side failure to collect simulation state.
Dug tiles
record the currently removed rocks at each sample, including unchanged ones;
restored rocks disappear from that array. Stones sort by index. Coordinates come
from the actual entities, with the same stage-root transform as the player.

`trajectory_hash` covers all frames' player/stone/placed/dug positions plus command
outcomes, goal frame, termination, error and executed frame count. It excludes
scenario batch fields, sample stride, rendering and wall time, so it compares one
seed across headless/replay and across batch sizes. `execution_hash` still includes
the full scenario and internal player/stone velocities and obstacle state; its
value changes with batch metadata and schema. Schema-1 report hashes are not
comparable with schema-2 execution hashes. The separate reach report stays schema 1;
its pixel maxima now use each trial's measured tile size rather than a literal.

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


## Viewport scale comparisons

`measure-scales.py` runs 0.5, 0.75, 1.0, 1.25, 1.5, and 2.0. Each run includes
all three map sizes and both directions and approaches. Individual reports retain
all trials; `summary.json` retains every maximum and fixed-input comparison, plus
`hypothesis_equal_cell_reach_across_scales`. Pixel distances use each trial's measured `StageGridMetrics` tile size (currently `60 * scale`).
`examples/headless/reach-scale-summary.json` is the measured reference summary.

The separate flat-floor probe uses identical input frames at every scale: move at
60 (running) or 90 (standing), jump at 90, release jump at 91, stop at 180; observe
through frame 239. It compares constant scale with a 1.0 baseline, and a resize
from 1.0 to the requested scale at frame 105 during the jump. Results include
jump success, apex frame/position/rise, six-frame-stable landing frame/position,
apex and landing differences in cells, and the frame-105 position delta in cells
and px. The excess cell delta subtracts ordinary motion in the baseline. The
resize keeps the stage centered; OS window placement and letterboxing are not
simulated. The probe uses the same scenario `scale_changes` driver as verification
and replay. A regression test checks apex rise and landing coordinates within
0.01 cells for all five changes, map sizes, directions, and approaches.

The registration regression test builds `StageScenePlugin` without running its
window/UI systems and compares it with the initialized headless app. It compares
actual system TypeIds, membership in `StageSystemSet`, and dependency reachability
(including within-set chains), plus fixed physics/physics/substep schedules.
Both paths also install the shared gameplay and physics plugins. Tests do not
require Bevy debug system names, a display server, or a renderer.


## Reference results

On the recorded build, Stage 1 / seed 17 clears at frame 787. Stage 2 / seed 17
clears at frame 1087 after the stone's `move("down");` reports `blocked` at frame
50. Both scenarios and reports are included here; the terrain was not edited to
force these outcomes.

The six-scale sweep (dev build) found one-cell steps and two-cell gaps for every
map size, direction, and approach. Identical-input apex/landing positions differ
by less than 0.00001 cells across constant scales. The shared gameplay system now
converts world-space motion when the viewport scale changes. The updated summary
records both `resize_apex_delta_cells` and `resize_landing_delta_cells`; the latter
compares both coordinates of the six-frame-stable landing. World pixel positions
change with the root scale while cell-space trajectories are preserved. Across
all five mid-jump changes and twelve map/direction/approach combinations, the
maximum apex-rise difference is 0.000003815 cells and the maximum landing-coordinate
difference is 0.000001192 cells. All constant-scale probe observations and reach
measurements match the previous reference summary exactly.


The schema-2 release benchmark completed 1,000 seeds × 1,800 frames with `--jobs 24` in
66.129 seconds of process wall time (66.062 seconds simulation time; 27,247.3 FPS).
One seed took 0.452 seconds wall time (0.434 seconds simulation; 4,150.4 FPS).
See `benchmark-summary.json`; regenerate with `python3 examples/headless/benchmark.py --jobs 24`.
The 300-second target is met on this machine and is not a CI timing gate.

## Placement and digging examples

```sh
env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify examples/headless/place.json --report examples/headless/place-report.json
env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify examples/headless/dig.json --report examples/headless/dig-report.json
```

`place.json` uses the existing Type4 `tools/stage_sim/examples/ai-candidate.ron`
without changing it and executes `place("up");`. `dig.json` uses main Stage 9
(Type3) and executes `dig("down");`. Both use seed 17, idle player input, and a
180-frame limit; clearing the stage is not their purpose. Their reports demonstrate
actual product commands and sampled platform/dug positions. A regression test also
marks a placed tile as dug and restores it, checking that it moves between the two
report arrays without appearing in both.

With the recorded seed, placement completes at frame 1; the first sampled platform
is at frame 10, cell `[10, 5]`, world `[-270, -270]` px. Digging completes at frame
30; that sample contains the removed rock at cell `[6, 2]`, world `[-510, -450]` px.
Both terminate with `max_frames`, as intended. No combined place-then-dig scenario
was forced: Type4 permits place but not dig, and Type3 permits dig but not place.
The component-lifecycle regression test covers removal/restoration of placed tiles
without changing those product capability restrictions.

## Player motion baselines

`motion-baselines.json` fixes every-frame trajectory hashes measured before
player parameterization, from main `14d8617ca764121eaa0e53055e4705deb90aad53`.
The synthetic RON maps below live only in examples; product stages are unchanged.

| Scenario | Input protocol (60 updates/s) |
| --- | --- |
| `motion-walk-stop.json` | Settle 60 frames; walk right until 120; observe rest through 359. |
| `motion-standing-jump.json` | Settle; jump at 90, release at 91; no horizontal input. |
| `motion-running-jump.json` | Right at 60; jump at 90, release at 91; stop at 180. |
| `motion-fall.json` | Right at 60 off a two-cell ledge; stop at 220. |
| `motion-wall-hit.json` | Right at 60 into a six-cell-high wall; release at 300. |
| `motion-goal-descent.json` | Stage 1 inputs, then observe 320 frames after first goal contact. |

`post_goal_frames` is optional (default 0). It extends observation after the first
goal contact, bounded by `max_frames`, without changing any product gameplay.
The descent example includes all 280 descent updates, the snap to the target and
restored gravity. Existing Stage 1/2 scenarios still stop at first goal contact.
`cargo test default_player_motion_matches_pre_parameterization_baselines` checks
these six examples plus Stage 1 and Stage 2 against the recorded hashes.
