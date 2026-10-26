# Headless autonomous agent v1

```sh
cargo build --release
env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-agent examples/headless/agent-stage-1.json \
  --report target/agent.json --jobs 24 --agent-attempts 8
# Each cleared seed is an ordinary scenario, with its generated seed_inputs:
env -u DISPLAY -u WAYLAND_DISPLAY target/release/keystone-cc \
  --headless-verify target/agent.solutions/seed-17.json --report target/verified.json
# On a machine with a working display, use --replay on the same solution file.
```

The input uses the existing strict headless scenario schema. Omit `inputs` and
`seed_inputs`. `stage_id`/`ron_path`, seed range, programs (Rhai or Keystone),
`player_physics`, constant `viewport_scale`, `start_frame`, `max_frames`, sampling
and post-goal observation retain their existing meaning. `--agent-attempts` is
1–64 (default 8); `--jobs` is 1–256. `max_frames` bounds **each attempt** and the
idle stone observation. v1 rejects `scale_changes` rather than applying reach
measured at the wrong scale. No window or external control server is started.

A report represents one condition: `no_code` when `programs` is empty, otherwise
`supplied_code`. Run the same scenario with and without programs to compare the
conditions. The report includes condition, clear rate, unsolved/error counts,
wall-clock simulation time (including startup reach measurement), mean attempts
and mean attempted frames. Per-seed `frames` sums all attempts;
`observation_frames` separately counts the idle stone forecast. Reach trial frames
are shared across the batch and excluded from those per-seed totals.

## Planning, control and retries

* Terrain ASCII supplies a bottom-up cell grid. Dijkstra searches supported cells
  and goal cells, considering walk, upward step jump, gap jump and drop edges.
  Stones and placed platforms supply support cells; observed dug tiles remove
  support. There is no ladder tile/control in the current product.
* Before processing the batch, the existing `--headless-measure-reach` landing
  trial function is run with the **resolved scenario physics and scale**. The
  agent uses both directions, running approaches, step sizes 0–5, gaps 0–8 and
  all five takeoff positions. Successful takeoffs are retained, not inferred from
  a ballistic formula or hard-coded one/two-cell limits. These are empirical
  bounds for a finite protocol, not an exhaustive capability proof.
* Jump clearance uses the maximum peak rise of successful landing trials,
  rounded up to whole cells (two cells with the default physics). The grid must
  be clear from the source through the destination column up to that height,
  including the source's headroom and intermediate cells. At the destination,
  the check starts at landing height, allowing an upward step's supporting floor.
  Drops include every destination-column cell from landing through departure
  height. Both checks use the same terrain, stone and placed-platform occupancy
  as support checks, including observed digging.
* For supplied programs, a fresh idle-player Session runs through the frame
  budget first. Stone/placed/dug occupied-cell changes retain their observed
  frames. If the current arrangement has no route, a future arrangement can
  propose an explicit wait until that frame. Actual support is sensed again
  before starting a segment. Forecasts are hypotheses: player-dependent scripts
  can behave differently during execution.
* Every attempt owns a new Session. After settling, the controller senses player
  position/velocity, the product foot shapecast, stones and platforms each frame.
  It steers toward each target, triggers a single jump at a measured takeoff,
  waits for support/landing and applies a distance/speed-dependent segment
  timeout. It can board a reachable stone when no complete route is known and
  wait for touch-sensitive behavior. It does not edit the world or restore a
  snapshot.
* Failure records retain attempt, segment index/action/endpoints, frame, player
  position and reason. A new attempt changes the failed edge's takeoff, penalizes
  that edge, then excludes it when measured variants are exhausted. If no route
  or exploratory action remains, identical retries stop early. The count is an
  upper bound, not a promise to execute redundant attempts.
* Only the product goal event produces `cleared`. `unsolved` means this bounded
  search did not clear, never that a map is impossible. Generation, compilation,
  script execution and observation failures produce `error`.

## Output and limitations

Each cleared seed retains `solution` in the report and writes an ordinary JSON
scenario to `<report-stem>.solutions/seed-N.json`. Replay/verification must run
from the same working directory as the original scenario (RON paths keep their
existing relative-path semantics). The last physical `result` contains terrain and
trajectory hashes, sampled trajectory and stone command outcomes, including
issued/completed frames and blocked/pending states. This successful input plus
program/outcome record is the handoff for the next PR's transferability analysis;
no transferability score is computed here. For an unsolved attempt stopped early
by the controller, `result` is a Session prefix: its inherited `max_frames`
termination label does not mean the full budget ran. Use `frames` and `failures`
for the actual stopping boundary and reason.

The grid checks a conservative rectangular jump passage, not the complete arc
or swept capsule. It can reject a physically possible jump around an obstacle
inside that rectangle. Upward jumps require an adjacent destination; larger
horizontal jumps use measured gap trials. Obstacles and dynamic solids are
static blockers at each observed frame. Subcell clearance, moving supports,
touch-sensitive programs, large parameter overrides and routes outside measured
bounds can still remain unsolved. A forecast is not a certificate that the
passage stays clear or support persists at arrival. The controller and retries
may recover from this coarse model; all successful routes still require
real-physics goal contact.

Fixture scenarios `agent-walk`, `agent-step`, `agent-gap`, `agent-high-no-code`,
`agent-high`, `agent-move`, `agent-corridor` and `agent-corridor-no-code` cover the
acceptance cases independently of product stages. The corridor has a one-cell
ceiling and requires `move("down");` to fill a floor hole before walking through;
without code it remains unsolved. `agent-stage-1` uses the frozen chunk fixture
over varying seeds. Rust
regressions replay successful `seed_inputs` through the ordinary verifier and
compare the complete `trajectory_hash`.

```sh
python3 examples/headless/agent-benchmark.py --jobs 24
# Also recheck all eight motion hashes, Stage 1/2 and every reach summary value:
python3 examples/headless/snapshot-baselines.py --binary target/release/keystone-cc
```

The benchmark defaults to 1,000 seeds, 1,800 frames/attempt and eight attempts.
It writes the scenario, full report, solutions and a compact summary beneath
`target/agent-benchmark`. Process wall time includes reach measurement and JSON
output. The 300-second target is recorded, never enforced as a CI timing gate.

## Recorded regression results

On the implementation build (seed 17; frame counts sum all attempts):

| Fixture / program | Result | Attempts | Frames |
| --- | --- | ---: | ---: |
| Walk / no code | cleared | 1 | 727 |
| One-cell step / no code | cleared | 1 | 744 |
| Two-cell gap / no code | cleared | 2 | 1,150 |
| Two-cell step / no code | unsolved | 1 | 30 |
| Same two-cell step / `place("up");` | cleared | 2 | 1,137 |
| Moving stone / `move("right");` | cleared | 1 | 759 |

The gap and placed-platform trials exercise actual failed segments followed by
fresh-world retries. Frozen Stage 1 seeds 17–20 all clear in two attempts and
produce more than one terrain hash. Every successful fixture and Stage 1 input
schedule reproduces its complete trajectory hash in the ordinary verifier.
Additional regressions cover a five-second wait for stone movement, lower jump
speed, compile/runtime script errors, retry/frame budgets and post-goal tails.

Release benchmark: 1,000 seeds (0–999), no code, `--jobs 24`, 1,800
frames/attempt, at most eight attempts: **85.691 seconds** process wall time,
including **17.099 seconds** of shared reach measurement. All 1,000 seeds cleared
(28 distinct terrain hashes), with zero unsolved/errors, **2.313 attempts** and
**1,430.408 attempted frames** per terrain on average. There were 758 two-attempt,
171 three-attempt and 71 four-attempt runs. The five-minute target was met without
changing product physics or the release optimization settings. See
[agent-benchmark-summary.json](agent-benchmark-summary.json).

All 1,000 exported solution files were then combined via `seed_inputs` and run
through release `--headless-verify --jobs 24`: all cleared and all 1,000
`trajectory_hash` values matched. The eight frozen motion hashes also matched in
the release binary, in addition to the full debug baseline/reach sweep.
