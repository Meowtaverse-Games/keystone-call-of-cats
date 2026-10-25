//! A bounded, empirical planner/controller. A route is only a hypothesis; only
//! Session's product goal event can certify a clear. Every retry owns a new world.
use super::*;
use checkpoint::{KeyChange, SensedState, Session};
use reach::AgentReach;
use std::collections::{BTreeSet, BinaryHeap};

type Cell = [i32; 2];
type Edge = (Cell, Cell);

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Action {
    Walk,
    JumpStep,
    JumpGap,
    Drop,
}
#[derive(Clone, Serialize, Debug)]
struct Segment {
    from: Cell,
    to: Cell,
    action: Action,
    takeoff_before_edge: f32,
}
#[derive(Clone)]
struct Grid {
    solid: BTreeSet<Cell>,
    goals: BTreeSet<Cell>,
    width: i32,
    height: i32,
}
impl Grid {
    fn new(ascii: &str) -> Self {
        let lines: Vec<_> = ascii.lines().collect();
        let mut grid = Self {
            solid: BTreeSet::new(),
            goals: BTreeSet::new(),
            width: lines.first().map_or(0, |s| s.len()) as i32,
            height: lines.len() as i32,
        };
        for (y, line) in lines.iter().rev().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let cell = [x as i32, y as i32];
                match ch {
                    '*' | '#' | 'O' | '?' => {
                        grid.solid.insert(cell);
                    }
                    'G' => {
                        grid.goals.insert(cell);
                    }
                    _ => {}
                }
            }
        }
        grid
    }
    fn observed(&self, state: &SensedState) -> Self {
        let mut grid = self.clone();
        for tile in &state.scene.dug_tiles {
            grid.solid.remove(&tile.cell);
        }
        for tile in &state.scene.placed_tiles {
            grid.solid.insert(tile.cell);
        }
        for stone in &state.scene.stones {
            grid.solid.insert(stone.position.cell);
        }
        grid
    }
    fn free(&self, [x, y]: Cell) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height && !self.solid.contains(&[x, y])
    }
    fn standing(&self, [x, y]: Cell) -> bool {
        self.free([x, y]) && self.solid.contains(&[x, y - 1])
    }
    fn route(
        &self,
        start: Cell,
        reach: &AgentReach,
        failures: &BTreeMap<Edge, usize>,
    ) -> Option<Vec<Segment>> {
        let mut queue = BinaryHeap::from([std::cmp::Reverse((0, start))]);
        let mut cost = BTreeMap::from([(start, 0)]);
        let mut previous: BTreeMap<Cell, Segment> = BTreeMap::new();
        while let Some(std::cmp::Reverse((distance, from))) = queue.pop() {
            if distance != cost[&from] {
                continue;
            }
            if self.goals.contains(&from) {
                let mut route = vec![];
                let mut cursor = from;
                while cursor != start {
                    let segment = previous[&cursor].clone();
                    cursor = segment.from;
                    route.push(segment);
                }
                route.reverse();
                return Some(route);
            }
            for dx in -(reach.max_gap() + 1)..=reach.max_gap() + 1 {
                if dx == 0 {
                    continue;
                }
                for y in 0..self.height {
                    let to = [from[0] + dx, y];
                    if !(self.standing(to) || self.goals.contains(&to) && self.free(to)) {
                        continue;
                    }
                    let dy = y - from[1];
                    let left = dx < 0;
                    let gap = dx.abs() - 1;
                    let walk = dx.abs() == 1 && dy == 0;
                    let drop = dx.abs() == 1 && dy < 0;
                    let mut takeoffs = if dy > 0 {
                        // A measured step has an adjacent destination. Wider upward
                        // jumps are deliberately left for a later search model.
                        if dx.abs() != 1 {
                            continue;
                        }
                        reach.takeoffs(left, true, dy)
                    } else {
                        reach.takeoffs(left, false, gap)
                    };
                    if !walk && !drop && takeoffs.is_empty() {
                        continue;
                    }
                    if drop && (y..from[1]).any(|h| !self.free([to[0], h])) {
                        continue;
                    }
                    if gap > 0
                        && (1..dx.abs()).any(|i| !self.free([from[0] + i * dx.signum(), from[1]]))
                    {
                        continue;
                    }
                    if walk || drop {
                        takeoffs = vec![0.5];
                    }
                    let tried = failures.get(&(from, to)).copied().unwrap_or(0);
                    if tried >= takeoffs.len() {
                        continue;
                    }
                    let action = if walk {
                        Action::Walk
                    } else if drop {
                        Action::Drop
                    } else if dy > 0 {
                        Action::JumpStep
                    } else {
                        Action::JumpGap
                    };
                    let next = distance
                        + dx.abs() * 10
                        + dy.abs() * 2
                        + if walk { 0 } else { 8 }
                        + tried as i32 * 20;
                    if next < *cost.get(&to).unwrap_or(&i32::MAX) {
                        cost.insert(to, next);
                        previous.insert(
                            to,
                            Segment {
                                from,
                                to,
                                action,
                                takeoff_before_edge: takeoffs[tried],
                            },
                        );
                        queue.push(std::cmp::Reverse((next, to)));
                    }
                }
            }
        }
        None
    }
}

#[derive(Serialize)]
struct Failure {
    attempt: usize,
    segment_index: usize,
    segment: Option<Segment>,
    frame: u64,
    player_cell: [f32; 2],
    reason: String,
}
#[derive(Serialize)]
struct Attempt {
    frames: u64,
    planned_wait_until: u64,
    route: Vec<Segment>,
    termination: String,
}
#[derive(Serialize)]
pub(super) struct AgentSeed {
    seed: u64,
    status: &'static str,
    reason: Option<String>,
    attempts: usize,
    frames: u64,
    observation_frames: u64,
    terrain_hash: Option<String>,
    failures: Vec<Failure>,
    runs: Vec<Attempt>,
    /// Last physical execution (successful execution when cleared).
    result: Option<SeedReport>,
    /// Ordinary headless schema, suitable for --headless-verify and --replay.
    solution: Option<Scenario>,
}
#[derive(Serialize)]
pub(super) struct AgentReport {
    schema_version: u32,
    condition: &'static str,
    player_physics: PlayerPhysicsParams,
    measured_reach: AgentReach,
    reach_measurement_seconds: f64,
    max_attempts: usize,
    count: usize,
    cleared: usize,
    unsolved: usize,
    errors: usize,
    clear_rate: f64,
    mean_attempts: f64,
    mean_frames: f64,
    elapsed_seconds: f64,
    seeds: Vec<AgentSeed>,
}

struct Forecast {
    frame: u64,
    grid: Grid,
}
fn cell(state: &SensedState) -> Cell {
    state.player_cell.map(|v| v.floor() as i32)
}
fn plan(
    base: &Grid,
    state: &SensedState,
    future: &[Forecast],
    reach: &AgentReach,
    failures: &BTreeMap<Edge, usize>,
) -> Option<(u64, Vec<Segment>)> {
    if let Some(route) = base.observed(state).route(cell(state), reach, failures) {
        return Some((state.frame, route));
    }
    // Waiting is explicit: a route through a future stone/platform arrangement
    // does not imply that it exists now. The controller rechecks it after waiting.
    for epoch in future.iter().filter(|e| e.frame > state.frame) {
        if epoch.grid.standing(cell(state))
            && let Some(route) = epoch.grid.route(cell(state), reach, failures)
        {
            return Some((epoch.frame, route));
        }
    }
    None
}

fn solve(
    scenario: &Scenario,
    seed: u64,
    reach: &AgentReach,
    max_attempts: usize,
) -> Result<AgentSeed> {
    let initial = Session::new(scenario.clone(), seed)?;
    let initial_report = initial.report()?;
    let base = Grid::new(&initial_report.terrain_ascii);
    drop(initial);
    let mut output = AgentSeed {
        seed,
        status: "unsolved",
        reason: None,
        attempts: 0,
        frames: 0,
        observation_frames: 0,
        terrain_hash: Some(initial_report.terrain_hash),
        failures: vec![],
        runs: vec![],
        result: None,
        solution: None,
    };
    let mut future = vec![];
    if !scenario.programs.is_empty() {
        let mut session = Session::new(scenario.clone(), seed)?;
        loop {
            let ended = session.step(&[])?;
            let observed = session.observe()?;
            let grid = base.observed(&observed);
            if future
                .last()
                .is_none_or(|f: &Forecast| f.grid.solid != grid.solid)
            {
                future.push(Forecast {
                    frame: session.frame(),
                    grid,
                });
            }
            if ended {
                break;
            }
        }
        let report = session.report()?;
        output.observation_frames = report.frames;
        if report.error.is_some() {
            output.status = "error";
            output.reason = report.error.clone();
            output.result = Some(report);
            return Ok(output);
        }
    }
    let speed = scenario.player_physics.resolve()?.speed;
    let mut failures = BTreeMap::new();
    for attempt in 1..=max_attempts {
        let mut session = Session::new(scenario.clone(), seed)?;
        let mut events = vec![];
        let mut held = [false; 3];
        let mut route: Vec<Segment> = vec![];
        let mut segment_index = 0;
        let mut segment_started = 0;
        let mut jumped = false;
        let mut wait_until = scenario.start_frame.saturating_add(30);
        let mut planned_wait = wait_until;
        let mut failure_reason = None;
        let mut state = session.observe()?;
        let mut visited_stones = BTreeSet::new();
        let mut next_plan = 0;
        loop {
            let frame = session.frame();
            let mut desired = [false; 3];
            if !state.goal_reached && frame >= wait_until {
                if route.is_empty() && frame >= next_plan {
                    next_plan = frame + 15;
                    if let Some((ready, proposed)) = plan(&base, &state, &future, reach, &failures)
                    {
                        wait_until = ready;
                        planned_wait = ready;
                        route = proposed;
                        segment_started = ready;
                    } else {
                        // No route is a limitation of this bounded model. Still
                        // step the real world: touched programs may change it.
                        failure_reason = Some("no route in measured grid model; player-dependent scripts or dynamic terrain may require a different policy".to_string());
                        if scenario.programs.is_empty() && state.grounded {
                            break;
                        }
                        // Try boarding a reachable stone once, then wait for its
                        // touch-sensitive program while periodically sensing anew.
                        for stone in &state.scene.stones {
                            if visited_stones.contains(&stone.stone_index) {
                                continue;
                            }
                            let mut boarding = base.observed(&state);
                            let [x, y] = stone.position.cell;
                            boarding.goals = BTreeSet::from([[x, y + 1]]);
                            if let Some(proposed) = boarding.route(cell(&state), reach, &failures) {
                                visited_stones.insert(stone.stone_index);
                                route = proposed;
                                segment_started = frame;
                                break;
                            }
                        }
                    }
                }
                if frame >= wait_until
                    && let Some(segment) = route.get(segment_index)
                {
                    let target_x = segment.to[0] as f32 + 0.5;
                    let dx = target_x - state.player_cell[0];
                    let landed = state.grounded && cell(&state)[1] == segment.to[1];
                    if dx.abs() < 0.12 && landed && (!jumped || frame > segment_started + 10) {
                        segment_index += 1;
                        segment_started = frame;
                        jumped = false;
                    } else {
                        let support_ready = matches!(segment.action, Action::Drop)
                            || base.goals.contains(&segment.to)
                            || base.observed(&state).standing(segment.to);
                        desired[0] = dx < -0.06;
                        desired[1] = dx > 0.06;
                        if matches!(segment.action, Action::JumpStep | Action::JumpGap) && !jumped {
                            let direction = (segment.to[0] - segment.from[0]).signum() as f32;
                            let edge =
                                segment.from[0] as f32 + if direction > 0.0 { 1.0 } else { 0.0 };
                            let takeoff = edge - direction * segment.takeoff_before_edge;
                            let offset = takeoff - state.player_cell[0];
                            // Some successful trials take off before the source
                            // cell's center. Actually back up to that position;
                            // jumping after passing it would not try that variant.
                            desired[0] = offset < -0.035;
                            desired[1] = offset > 0.035;
                            if offset.abs() <= 0.035 && state.grounded && support_ready {
                                desired[0] = direction < 0.0;
                                desired[1] = direction > 0.0;
                                desired[2] = true;
                                jumped = true;
                            }
                        }
                        let timeout = ((segment.to[0] - segment.from[0]).unsigned_abs() as u64)
                            .saturating_mul((3600.0 / speed).ceil() as u64)
                            .saturating_add(150);
                        if frame.saturating_sub(segment_started) > timeout {
                            failure_reason = Some(if state.on_stone.is_some() {
                                "segment timeout while riding/waiting on stone; player-dependent motion unresolved"
                            } else { "segment timeout: landing or target not reached" }.into());
                            break;
                        }
                        // When the platform is carrying us, keep our feet on it
                        // while the intended destination is above measured reach.
                        if !jumped && !support_ready {
                            desired = [false; 3];
                        }
                    }
                }
            }
            let mut changes = vec![];
            for (index, key) in [InputKey::ArrowLeft, InputKey::ArrowRight, InputKey::Space]
                .into_iter()
                .enumerate()
            {
                if desired[index] != held[index] {
                    changes.push(KeyChange {
                        key,
                        pressed: desired[index],
                    });
                    events.push(InputEvent {
                        frame,
                        key,
                        pressed: desired[index],
                    });
                }
            }
            held = desired;
            let ended = session.step(&changes)?;
            state = session.observe()?;
            if ended {
                break;
            }
            if !route.is_empty() && segment_index == route.len() {
                // The goal event, never reaching the last planned cell, decides
                // success. Retry planning if the physical goal is still pending.
                route.clear();
                segment_index = 0;
            }
        }
        let report = session.report()?;
        output.attempts = attempt;
        output.frames += report.frames;
        output.runs.push(Attempt {
            frames: report.frames,
            planned_wait_until: planned_wait,
            route: route.clone(),
            termination: report.termination.clone(),
        });
        if report.cleared {
            let mut solution = scenario.clone();
            solution.seed_start = seed;
            solution.seed_count = 1;
            solution.seed_inputs.insert(seed, events);
            output.solution = Some(solution);
            output.status = "cleared";
            output.reason = None;
            output.result = Some(report);
            break;
        }
        if report.error.is_some() {
            output.status = "error";
            output.reason = report.error.clone();
            output.result = Some(report);
            break;
        }
        let segment = route.get(segment_index).cloned();
        let reason = failure_reason.unwrap_or_else(|| "attempt frame budget exhausted".into());
        output.failures.push(Failure {
            attempt,
            segment_index,
            segment: segment.clone(),
            frame: session.frame(),
            player_cell: state.player_cell,
            reason: reason.clone(),
        });
        output.reason = Some(reason);
        output.result = Some(report);
        if let Some(segment) = segment {
            *failures.entry((segment.from, segment.to)).or_insert(0) += 1;
        } else {
            // All observed arrangements were exhausted without a candidate.
            // Replaying identical idle input cannot add information.
            break;
        }
    }
    Ok(output)
}

pub(super) fn run(scenario: &Scenario, jobs: usize, max_attempts: usize) -> Result<AgentReport> {
    validate(scenario)?;
    if !(1..=256).contains(&jobs) || !(1..=64).contains(&max_attempts) {
        return Err("jobs must be 1..=256 and --agent-attempts must be 1..=64".into());
    }
    if !scenario.inputs.is_empty() || !scenario.seed_inputs.is_empty() {
        return Err("Agent scenarios must omit player inputs and seed_inputs".into());
    }
    if !scenario.scale_changes.is_empty() {
        return Err("Agent v1 requires a constant viewport_scale (scale_changes are supported by verify/replay only)".into());
    }
    let started = Instant::now();
    let player_physics = scenario.player_physics.resolve()?;
    let measured_reach = reach::measure_agent(scenario.viewport_scale, &player_physics)?;
    let reach_measurement_seconds = started.elapsed().as_secs_f64();
    let next = AtomicUsize::new(0);
    let mut seeds = std::thread::scope(|scope| -> Result<Vec<AgentSeed>> {
        let mut workers = vec![];
        for _ in 0..jobs.min(scenario.seed_count) {
            workers.push(scope.spawn(|| {
                let mut results = vec![];
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= scenario.seed_count {
                        break;
                    }
                    let seed = scenario.seed_start + index as u64;
                    // A failed generation/observation is per terrain, never a false
                    // unsolved result and never an excuse to discard other seeds.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        solve(scenario, seed, &measured_reach, max_attempts)
                    }))
                    .unwrap_or_else(|_| Err("terrain generation or simulation panicked".into()));
                    results.push(result.unwrap_or_else(|error| AgentSeed {
                        seed,
                        status: "error",
                        reason: Some(error.to_string()),
                        attempts: 0,
                        frames: 0,
                        observation_frames: 0,
                        terrain_hash: None,
                        failures: vec![],
                        runs: vec![],
                        result: None,
                        solution: None,
                    }));
                }
                results
            }));
        }
        let mut results = vec![];
        for worker in workers {
            results.extend(worker.join().map_err(|_| "Agent worker panicked")?);
        }
        Ok(results)
    })?;
    seeds.sort_by_key(|s| s.seed);
    let count = seeds.len();
    let cleared = seeds.iter().filter(|s| s.status == "cleared").count();
    Ok(AgentReport {
        schema_version: 1,
        condition: if scenario.programs.is_empty() {
            "no_code"
        } else {
            "supplied_code"
        },
        player_physics,
        measured_reach,
        reach_measurement_seconds,
        max_attempts,
        count,
        cleared,
        unsolved: seeds.iter().filter(|s| s.status == "unsolved").count(),
        errors: seeds.iter().filter(|s| s.status == "error").count(),
        clear_rate: cleared as f64 / count as f64,
        mean_attempts: seeds.iter().map(|s| s.attempts).sum::<usize>() as f64 / count as f64,
        mean_frames: seeds.iter().map(|s| s.frames).sum::<u64>() as f64 / count as f64,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        seeds,
    })
}

pub(super) fn write_solutions(report_path: &std::path::Path, report: &AgentReport) -> Result<()> {
    let directory = report_path.with_extension("solutions");
    if report.cleared > 0 {
        fs::create_dir_all(&directory)?;
    }
    for seed in &report.seeds {
        if let Some(solution) = &seed.solution {
            fs::write(
                directory.join(format!("seed-{}.json", seed.seed)),
                serde_json::to_string_pretty(solution)? + "\n",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
