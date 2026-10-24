//! Offline driver: all simulation is performed by the product's stage systems.
mod reach;

use crate::util::script_types::{ScriptCommand, ScriptExecutionError, ScriptProgram, ScriptState};
use crate::{
    resources::{
        asset_store::AssetStore,
        chunk_grammar_map::{self, ChunkGrammarConfig, Map, TileKind},
        design_resolution::ScaledViewport,
        game_state::GameState,
        launch_profile::{LaunchProfile, LaunchType},
        script_engine::{Language, ScriptExecutor},
        settings::GameSettings,
        stone_type::{StoneCapabilities, StoneType},
        tiled::{TiledMapAssets, TiledTilesetImage, Tileset},
    },
    scenes::{
        assets::PLAYER_IDLE_KEYS,
        stage::{
            StageGameplayPlugin,
            components::{Player, StoneIndex, StoneRune},
            systems::{
                self, ScriptEditorState, StageAudioHandles, StageAudioState, StageGridMetrics,
                StageSystemSet, StoneAppendCommandMessage, StoneExternalOutcomeMessage,
            },
        },
    },
};
use avian2d::prelude::*;
use bevy::{
    app::{TaskPoolOptions, TaskPoolPlugin},
    asset::AssetPlugin,
    ecs::{
        message::MessageMutator,
        schedule::{Schedules, SingleThreadedExecutor},
    },
    gizmos::{GizmoAsset, config::DefaultGizmoConfigGroup},
    prelude::*,
    state::app::StatesPlugin,
    time::TimeUpdateStrategy,
};
use bevy_fluent::prelude::Localization;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub stage_id: Option<usize>,
    pub ron_path: Option<PathBuf>,
    pub seed_start: u64,
    pub seed_count: usize,
    #[serde(default)]
    pub programs: Vec<StoneProgram>,
    #[serde(default)]
    pub start_frame: u64,
    #[serde(default)]
    pub inputs: Vec<InputEvent>,
    #[serde(default)]
    pub seed_inputs: BTreeMap<u64, Vec<InputEvent>>,
    pub max_frames: u64,
    #[serde(default = "default_stride")]
    pub trajectory_every: u64,
}
fn default_stride() -> u64 {
    10
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoneProgram {
    pub stone_index: usize,
    pub language: Language,
    pub code: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputEvent {
    pub frame: u64,
    pub key: InputKey,
    pub pressed: bool,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum InputKey {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    KeyA,
    KeyD,
    KeyW,
    Space,
}
impl InputKey {
    fn code(self) -> KeyCode {
        match self {
            Self::ArrowLeft => KeyCode::ArrowLeft,
            Self::ArrowRight => KeyCode::ArrowRight,
            Self::ArrowUp => KeyCode::ArrowUp,
            Self::KeyA => KeyCode::KeyA,
            Self::KeyD => KeyCode::KeyD,
            Self::KeyW => KeyCode::KeyW,
            Self::Space => KeyCode::Space,
        }
    }
}
#[derive(Serialize)]
pub struct TrajectoryPoint {
    pub frame: u64,
    pub cell: [i32; 2],
    pub world_px: [f32; 2],
}
#[derive(Serialize)]
pub struct SeedReport {
    pub seed: u64,
    pub cleared: bool,
    pub goal_frame: Option<u64>,
    pub frames: u64,
    pub termination: String,
    pub error: Option<String>,
    pub terrain_ascii: String,
    pub terrain_hash: String,
    pub execution_hash: String,
    pub commands: Vec<CommandResult>,
    pub trajectory: Vec<TrajectoryPoint>,
}
#[derive(Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub seeds: Vec<SeedReport>,
    pub count: usize,
    pub cleared: usize,
    pub frames: u64,
    pub elapsed_seconds: f64,
    pub frames_per_second: f64,
}
#[derive(Clone, Serialize)]
pub struct CommandResult {
    pub action_id: u64,
    pub stone_index: usize,
    pub command: String,
    pub issued_frame: u64,
    pub completed_frame: Option<u64>,
    pub result: String,
}
#[derive(Resource, Default)]
struct Observation {
    runtime_error: Arc<Mutex<Option<String>>>,
    frame: u64,
    commands: Vec<CommandResult>,
}

/// Observe errors without replacing the product VM or its stepping logic.
struct ObservedProgram {
    inner: Box<dyn ScriptProgram>,
    error: Arc<Mutex<Option<String>>>,
}
impl ScriptProgram for ObservedProgram {
    fn next(&mut self, state: &ScriptState) -> Option<ScriptCommand> {
        self.inner.next(state)
    }
    fn take_error(&mut self) -> Option<ScriptExecutionError> {
        let error = self.inner.take_error();
        if let Some(error) = &error {
            *self.error.lock().expect("error observation lock") = Some(error.to_string());
        }
        error
    }
}

// The optional correlation IDs only enable the existing outcome messages; no server,
// ownership resource or external command injection is installed.
fn tag_commands(
    mut messages: MessageMutator<StoneAppendCommandMessage>,
    mut observation: ResMut<Observation>,
    editor: Res<ScriptEditorState>,
) {
    for message in messages.read() {
        if !editor.controls_enabled {
            continue;
        }
        let id = observation.commands.len() as u64;
        message.external_action_id = Some(id);
        let frame = observation.frame;
        observation.commands.push(CommandResult {
            action_id: id,
            stone_index: message.stone_index,
            command: format!("{:?}", message.command),
            issued_frame: frame,
            completed_frame: None,
            result: "pending".into(),
        });
    }
}
fn collect_outcomes(
    mut messages: MessageReader<StoneExternalOutcomeMessage>,
    mut observation: ResMut<Observation>,
) {
    for message in messages.read() {
        let frame = observation.frame;
        if let Some(command) = observation.commands.get_mut(message.action_id as usize) {
            command.completed_frame = Some(frame);
            command.result = if message.blocked {
                "blocked"
            } else {
                "completed"
            }
            .into();
        }
    }
}

fn setup(
    mut commands: Commands,
    map: Res<Map>,
    viewport: Res<ScaledViewport>,
    tiles: Res<TiledMapAssets>,
    store: Res<AssetStore>,
    server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    systems::spawn_stage(
        &mut commands,
        Transform::from_xyz(0.0, 0.0, 1.0),
        &tiles,
        &map,
        &viewport,
        &store,
        &server,
        &mut layouts,
    );
}

pub(super) fn build_app(map: Map) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions::with_num_threads(1),
        }),
        AssetPlugin::default(),
        TransformPlugin,
        StatesPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<TextureAtlasLayout>()
    .init_asset::<bevy::mesh::Mesh>()
    .init_asset::<GizmoAsset>()
    .init_gizmo_group::<DefaultGizmoConfigGroup>()
    .add_plugins((PhysicsPlugins::default(), StageGameplayPlugin))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .insert_resource(LaunchProfile::default())
    .insert_resource(ScaledViewport::new(Vec2::new(1800.0, 1200.0)))
    .insert_resource(Localization::new())
    .insert_resource(GameSettings::default())
    .insert_resource(StageAudioHandles::new(Handle::default(), Handle::default()))
    .init_resource::<StageAudioState>()
    .init_resource::<ScriptEditorState>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<Observation>()
    .insert_resource(TiledMapAssets {
        tileset: Tileset {
            image: Some(TiledTilesetImage {
                texture: Handle::default(),
                layout: Handle::default(),
                tile_size: UVec2::splat(16),
            }),
        },
    })
    .insert_resource(map)
    .init_state::<GameState>()
    .add_systems(
        Startup,
        (
            setup,
            crate::systems::engine::friction::apply_zero_friction_to_rigid_bodies.after(setup),
        ),
    )
    .add_systems(Update, systems::update_dynamic_solid)
    .add_systems(
        Update,
        tag_commands
            .after(StageSystemSet::Script)
            .before(StageSystemSet::Reset),
    )
    .add_systems(Last, collect_outcomes);
    let mut store = AssetStore::default();
    for key in PLAYER_IDLE_KEYS {
        store.insert_image(key.into(), Handle::default());
    }
    app.insert_resource(store);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Stage);
    app.finish();
    app.cleanup();
    // Same tree optimization algorithm, run inline: worlds already run in parallel
    // and should not serialize their per-step work through a shared async pool.
    app.world_mut()
        .resource_mut::<avian2d::collider_tree::ColliderTreeOptimization>()
        .use_async_tasks = false;
    // Independent worlds run on workers; avoid nesting ECS parallelism, including
    // Avian's FixedPostUpdate/PhysicsSchedule and its substep schedules.
    for (_, schedule) in app.world_mut().resource_mut::<Schedules>().iter_mut() {
        schedule.set_executor(SingleThreadedExecutor::new());
    }
    // Bevy's initial Time update has zero delta, just as in the graphical app.
    app.update();
    app
}

fn load_map(scenario: &Scenario, seed: u64) -> Result<Map> {
    let path = match (&scenario.stage_id, &scenario.ron_path) {
        (Some(id @ 1..=12), None) => PathBuf::from(format!("assets/stages/stage-{id}.ron")),
        (None, Some(path)) => path.clone(),
        _ => return Err("Specify exactly one of stage_id (1..12) or ron_path".into()),
    };
    let config: ChunkGrammarConfig = ron::from_str(&fs::read_to_string(path)?)?;
    Ok(chunk_grammar_map::generate_map_with_seed(config, seed))
}

fn start_programs(app: &mut App, scenario: &Scenario) -> Result<()> {
    let world = app.world_mut();
    let mut stones: Vec<_> = world
        .query_filtered::<(&StoneIndex, &StoneType), With<StoneRune>>()
        .iter(world)
        .map(|(i, t)| (i.0, *t))
        .collect();
    stones.sort_by_key(|(i, _)| *i);
    if scenario
        .programs
        .iter()
        .any(|p| p.stone_index >= stones.len())
    {
        return Err("Program refers to a missing stone".into());
    }
    let executor = ScriptExecutor::deterministic();
    let capabilities = StoneCapabilities::default();
    let mut programs = Vec::new();
    let mut buffers = Vec::new();
    for (idx, kind) in stones {
        let program = scenario.programs.iter().find(|p| p.stone_index == idx);
        let code = program.map_or("", |p| p.code.as_str());
        let language = program.map_or(Language::Rhai, |p| p.language);
        // Keystone's current VM owns an unseedable random generator. Reject the
        // identifier conservatively (also in comments) rather than claim determinism.
        if language == Language::Keystone
            && code
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|token| token == "rand")
        {
            return Err(
                "Keystone rand is not supported in deterministic headless scenarios".into(),
            );
        }

        let mut compiled = executor
            .compile_step(
                language,
                vec![code.into()],
                capabilities.get_capabilities(kind),
            )
            .map_err(|e| e.to_string())?;
        programs.push(compiled.pop().map(|inner| {
            Box::new(ObservedProgram {
                inner,
                error: world.resource::<Observation>().runtime_error.clone(),
            }) as Box<dyn ScriptProgram>
        }));
        buffers.push(code.into());
    }
    world.write_message(systems::StoneCommandMessage { commands: vec![] });
    let mut editor = world.resource_mut::<ScriptEditorState>();
    editor.active_programs = programs;
    editor.buffers = buffers;
    editor.controls_enabled = true;
    editor.pending_player_reset = true;
    editor.stage_cleared = false;
    editor.stage_clear_popup_open = false;
    Ok(())
}

// Versioned FNV-1a, deliberately independent of Rust's randomized/default hashers.
struct Digest(u64);
impl Digest {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn add(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn hex(&self) -> String {
        format!("fnv1a64-v1:{:016x}", self.0)
    }
}
fn hash(bytes: &[u8]) -> String {
    let mut digest = Digest::new();
    digest.add(bytes);
    digest.hex()
}

pub(super) fn player_position(app: &mut App) -> Option<Vec2> {
    let world = app.world_mut();
    world
        .query_filtered::<&GlobalTransform, With<Player>>()
        .iter(world)
        .next()
        .map(|t| t.translation().truncate())
}
fn snapshot(app: &mut App, frame: u64, digest: &mut Digest) -> Result<TrajectoryPoint> {
    let position = player_position(app).ok_or("Player was not spawned")?;
    digest.add(&frame.to_le_bytes());
    let world = app.world_mut();
    let mut bodies: Vec<_> = world
        .query::<(
            &GlobalTransform,
            &LinearVelocity,
            Option<&StoneIndex>,
            Option<&Player>,
        )>()
        .iter(world)
        .filter(|(_, _, stone, player)| stone.is_some() || player.is_some())
        .map(|(t, v, s, _)| (s.map_or(0, |s| s.0 + 1), t.translation(), v.0))
        .collect();
    bodies.sort_by_key(|b| b.0);
    for (index, pos, velocity) in bodies {
        digest.add(&(index as u64).to_le_bytes());
        for f in [pos.x, pos.y, velocity.x, velocity.y] {
            digest.add(&f.to_bits().to_le_bytes());
        }
    }
    // Include timed obstacle collision changes even when the player never touches
    // them, so their deterministic lifetime is part of the execution fingerprint.
    let mut obstacles: Vec<_> = world
        .query::<(
            &GlobalTransform,
            &systems::AnimatedObstacle,
            Option<&Collider>,
        )>()
        .iter(world)
        .map(|(t, o, c)| {
            (
                t.translation().x.to_bits(),
                t.translation().y.to_bits(),
                o.lifetime_timer.elapsed().as_nanos(),
                c.is_some(),
            )
        })
        .collect();
    obstacles.sort_unstable();
    for (x, y, elapsed, collider) in obstacles {
        digest.add(&x.to_le_bytes());
        digest.add(&y.to_le_bytes());
        digest.add(&elapsed.to_le_bytes());
        digest.add(&[u8::from(collider)]);
    }
    let metrics = world.resource::<StageGridMetrics>();
    let cell = ((position + metrics.viewport_size * 0.5) / metrics.local_tile_size)
        .floor()
        .as_ivec2();
    Ok(TrajectoryPoint {
        frame,
        cell: [cell.x, cell.y],
        world_px: [position.x, position.y],
    })
}

fn verify_seed(scenario: &Scenario, seed: u64) -> Result<SeedReport> {
    let map = load_map(scenario, seed)?;
    let ascii = chunk_grammar_map::ascii_map(&map);
    let mut digest = Digest::new();
    digest.add(&serde_json::to_vec(scenario)?);
    digest.add(&seed.to_le_bytes());
    digest.add(ascii.as_bytes());
    let mut app = build_app(map);
    let mut report = SeedReport {
        seed,
        cleared: false,
        goal_frame: None,
        frames: 0,
        termination: "max_frames".into(),
        error: None,
        terrain_hash: hash(ascii.as_bytes()),
        terrain_ascii: ascii,
        execution_hash: String::new(),
        commands: vec![],
        trajectory: vec![],
    };
    let mut inputs = scenario
        .seed_inputs
        .get(&seed)
        .unwrap_or(&scenario.inputs)
        .clone();
    inputs.sort_by_key(|e| e.frame);
    let mut cursor = 0;
    for frame in 0..scenario.max_frames {
        app.world_mut().resource_mut::<Observation>().frame = frame;
        if frame == scenario.start_frame
            && let Err(error) = start_programs(&mut app, scenario)
        {
            report.termination = "compile_error".into();
            report.error = Some(error.to_string());
            break;
        }
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear(); // Clear edges, preserve held keys. No InputPlugin overwrites injection.
            while let Some(event) = inputs.get(cursor).filter(|e| e.frame == frame) {
                if event.pressed {
                    keys.press(event.key.code());
                } else {
                    keys.release(event.key.code());
                }
                cursor += 1;
            }
        }
        app.update();
        report.frames += 1;
        let point = snapshot(&mut app, frame, &mut digest)?;
        let editor = app.world().resource::<ScriptEditorState>();
        if editor.stage_cleared {
            report.cleared = true;
            report.goal_frame = Some(frame);
            report.termination = "goal".into();
        } else if frame >= scenario.start_frame && !editor.controls_enabled {
            report.termination = "script_error".into();
            report.error = app
                .world()
                .resource::<Observation>()
                .runtime_error
                .lock()
                .expect("error observation lock")
                .clone()
                .or_else(|| editor.last_run_feedback.clone());
        }
        let ended = report.termination != "max_frames" || frame + 1 == scenario.max_frames;
        if frame % scenario.trajectory_every == 0 || ended {
            report.trajectory.push(point);
        }
        if ended {
            break;
        }
    }
    report.commands = std::mem::take(&mut app.world_mut().resource_mut::<Observation>().commands);
    digest.add(&serde_json::to_vec(&report)?);
    report.execution_hash = digest.hex();
    Ok(report)
}
fn validate(scenario: &Scenario) -> Result<()> {
    if scenario.seed_count == 0
        || scenario.max_frames == 0
        || scenario.trajectory_every == 0
        || scenario.start_frame >= scenario.max_frames
    {
        return Err("seed_count, max_frames and trajectory_every must be positive; start_frame must be below max_frames".into());
    }
    scenario
        .seed_start
        .checked_add((scenario.seed_count - 1) as u64)
        .ok_or("Seed range overflows u64")?;
    let mut indices = std::collections::HashSet::new();
    for p in &scenario.programs {
        if !indices.insert(p.stone_index) {
            return Err("Duplicate stone program".into());
        }
    }
    for event in scenario
        .inputs
        .iter()
        .chain(scenario.seed_inputs.values().flatten())
    {
        if event.frame >= scenario.max_frames {
            return Err("Input frame must be below max_frames".into());
        }
    }
    Ok(())
}
fn verify(scenario: &Scenario, jobs: usize) -> Result<Report> {
    validate(scenario)?;
    let started = Instant::now();
    let next = AtomicUsize::new(0);
    let mut seeds = std::thread::scope(|scope| -> Result<Vec<SeedReport>> {
        let mut workers = vec![];
        for _ in 0..jobs.min(scenario.seed_count) {
            workers.push(scope.spawn(|| -> Result<Vec<SeedReport>> {
                let mut results = vec![];
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= scenario.seed_count {
                        break;
                    }
                    results.push(verify_seed(scenario, scenario.seed_start + index as u64)?);
                }
                Ok(results)
            }));
        }
        let mut results = vec![];
        for worker in workers {
            results.extend(
                worker
                    .join()
                    .map_err(|_| "Verification worker panicked")??,
            );
        }
        Ok(results)
    })?;
    seeds.sort_by_key(|s| s.seed);
    let seconds = started.elapsed().as_secs_f64();
    let frames = seeds.iter().map(|s| s.frames).sum();
    Ok(Report {
        schema_version: 1,
        count: seeds.len(),
        cleared: seeds.iter().filter(|s| s.cleared).count(),
        frames,
        elapsed_seconds: seconds,
        frames_per_second: frames as f64 / seconds,
        seeds,
    })
}
pub fn run(profile: &LaunchProfile) -> Result<()> {
    let path = profile
        .headless_report
        .as_ref()
        .ok_or("--report <path> is required")?;
    let jobs = profile.headless_jobs.unwrap_or(1);
    if jobs == 0 || jobs > 256 {
        return Err("--jobs must be between 1 and 256".into());
    }
    let value = match profile.launch_type {
        LaunchType::HeadlessMeasureReach => serde_json::to_value(reach::measure()?)?,
        _ => {
            let source = profile
                .headless_scenario
                .as_ref()
                .ok_or("--headless-verify <scenario.json> is required")?;
            let scenario: Scenario = serde_json::from_str(&fs::read_to_string(source)?)?;
            serde_json::to_value(verify(&scenario, jobs)?)?
        }
    };
    fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scenario(stage: usize) -> Scenario {
        serde_json::from_value(serde_json::json!({"stage_id":stage,"seed_start":17,"seed_count":1,"max_frames":120,"inputs":[{"frame":2,"key":"ArrowRight","pressed":true},{"frame":20,"key":"Space","pressed":true}]})).unwrap()
    }
    #[test]
    fn repeat_runs_have_identical_hashes_and_product_physics_moves_player() {
        let s = scenario(1);
        let a = verify_seed(&s, 17).unwrap();
        let b = verify_seed(&s, 17).unwrap();
        assert_eq!(
            a.execution_hash,
            b.execution_hash,
            "first={}\nsecond={}",
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_ne!(
            a.trajectory.first().unwrap().world_px,
            a.trajectory.last().unwrap().world_px
        );
    }
    #[test]
    fn worker_count_does_not_change_results() {
        let mut s = scenario(1);
        s.seed_count = 3;
        let serial = verify(&s, 1).unwrap();
        let parallel = verify(&s, 3).unwrap();
        assert_eq!(
            serial
                .seeds
                .iter()
                .map(|s| &s.execution_hash)
                .collect::<Vec<_>>(),
            parallel
                .seeds
                .iter()
                .map(|s| &s.execution_hash)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn seeded_obstacle_lifetime_is_reproducible() {
        fn lifetimes(seed: u64) -> Vec<u32> {
            let mut map = Map::synthetic(
                (26, 10),
                vec![
                    ((1, 3), TileKind::PlayerSpawn),
                    ((5, 3), TileKind::Obstacle),
                    ((6, 3), TileKind::Obstacle),
                ],
            );
            map.simulation_seed = Some(seed);
            let mut app = build_app(map);
            let world = app.world_mut();
            world
                .query::<&systems::AnimatedObstacle>()
                .iter(world)
                .map(|o| o.lifetime_timer.duration().as_secs_f32().to_bits())
                .collect()
        }
        assert_eq!(lifetimes(3), lifetimes(3));
        assert_ne!(lifetimes(3), lifetimes(4));
    }

    #[test]
    fn non_yielding_rhai_is_a_deterministic_script_error() {
        let mut s = scenario(2);
        s.programs.push(StoneProgram {
            stone_index: 0,
            language: Language::Rhai,
            code: "loop {}".into(),
        });
        let a = verify_seed(&s, 17).unwrap();
        let b = verify_seed(&s, 17).unwrap();
        assert_eq!(a.termination, "script_error");
        assert_eq!(a.frames, 1);
        assert!(a.error.as_ref().unwrap().contains("100000 operations"));
        assert_eq!(a.execution_hash, b.execution_hash);
    }

    #[test]
    fn dynamic_solids_follow_a_reproducible_seeded_stream() {
        fn disabled_cells(seed: u64) -> Vec<Vec<(u32, u32)>> {
            let mut tiles = vec![((1, 3), TileKind::PlayerSpawn)];
            tiles.extend((5..11).map(|x| ((x, 1), TileKind::DynamicSolid)));
            let mut map = Map::synthetic((26, 10), tiles);
            map.simulation_seed = Some(seed);
            map.dynamic_min = 1;
            map.dynamic_max = 3;
            let mut app = build_app(map);
            (0..10)
                .map(|_| {
                    app.world_mut().write_message(systems::StoneTickMessage);
                    app.update();
                    let world = app.world_mut();
                    let mut cells: Vec<_> = world
                        .query_filtered::<&GlobalTransform, With<ColliderDisabled>>()
                        .iter(world)
                        .map(|t| (t.translation().x.to_bits(), t.translation().y.to_bits()))
                        .collect();
                    cells.sort_unstable();
                    cells
                })
                .collect()
        }
        let first = disabled_cells(17);
        assert!(first.iter().all(|cells| !cells.is_empty()));
        assert_eq!(first, disabled_cells(17));
        assert_ne!(first, disabled_cells(18));
    }

    #[test]
    fn scenario_rejects_ambiguous_or_invalid_input() {
        let mut s = scenario(1);
        s.trajectory_every = 0;
        assert!(validate(&s).is_err());
        s.trajectory_every = 1;
        s.seed_start = u64::MAX;
        s.seed_count = 2;
        assert!(validate(&s).is_err());
    }

    #[test]
    fn scripts_use_product_capabilities_and_outcomes() {
        let mut s = scenario(2);
        s.programs.push(StoneProgram {
            stone_index: 0,
            language: Language::Rhai,
            code: "move(\"down\");".into(),
        });
        let a = verify_seed(&s, 17).unwrap();
        let b = verify_seed(&s, 17).unwrap();
        assert_eq!(a.execution_hash, b.execution_hash);
        assert!(!a.commands.is_empty());
        assert!(a.commands.iter().all(|c| c.completed_frame.is_some()));
        s.programs[0].language = Language::Keystone;
        s.programs[0].code = "move down".into();
        let keystone = verify_seed(&s, 17).unwrap();
        assert_eq!(keystone.commands.len(), 1);
        assert!(keystone.commands[0].completed_frame.is_some());
        s.programs[0].code = "let x = rand(3)".into();
        assert_eq!(verify_seed(&s, 17).unwrap().termination, "compile_error");
        s.programs[0].language = Language::Rhai;
        s.programs[0].code = "dig(\"down\");".into();
        assert_eq!(verify_seed(&s, 17).unwrap().termination, "compile_error");
    }
}
