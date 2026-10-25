//! Offline driver: all simulation is performed by the product's stage systems.
mod reach;
pub mod replay;

use crate::util::script_types::{ScriptCommand, ScriptExecutionError, ScriptProgram, ScriptState};
use crate::{
    resources::{
        asset_store::AssetStore,
        chunk_grammar_map::{self, ChunkGrammarConfig, Map, TileKind},
        design_resolution::ScaledViewport,
        game_state::GameState,
        launch_profile::{LaunchProfile, LaunchType},
        player_physics::{PlayerPhysicsOverrides, PlayerPhysicsParams},
        script_engine::{Language, ScriptExecutor},
        settings::GameSettings,
        stage_catalog::{StageCatalog, StageId},
        stone_type::{StoneCapabilities, StoneType},
        tiled::{TiledMapAssets, TiledTilesetImage, Tileset},
    },
    scenes::{
        assets::PLAYER_IDLE_KEYS,
        stage::{
            StageGameplayPlugin, StagePhysicsPlugin,
            components::{Player, StageRoot, StoneIndex, StoneRune},
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
    #[serde(default)]
    pub player_physics: PlayerPhysicsOverrides,
    #[serde(default = "default_scale")]
    pub viewport_scale: f32,
    #[serde(default)]
    pub scale_changes: Vec<ScaleChange>,
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
    /// Optional observation tail for the product goal descent (default: stop at goal).
    #[serde(default)]
    pub post_goal_frames: u64,
    #[serde(default = "default_stride")]
    pub trajectory_every: u64,
}
fn default_scale() -> f32 {
    1.0
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaleChange {
    pub frame: u64,
    pub scale: f32,
}
fn valid_scale(scale: f32) -> bool {
    scale.is_finite() && scale > 0.0
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
pub struct PositionSample {
    pub cell: [i32; 2],
    pub world_px: [f32; 2],
}
#[derive(Serialize)]
pub struct StoneSample {
    pub stone_index: usize,
    #[serde(flatten)]
    pub position: PositionSample,
}
#[derive(Serialize)]
pub struct TrajectoryPoint {
    pub stones: Vec<StoneSample>,
    pub placed_tiles: Vec<PositionSample>,
    pub dug_tiles: Vec<PositionSample>,
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
    pub trajectory_hash: String,
    pub commands: Vec<CommandResult>,
    pub trajectory: Vec<TrajectoryPoint>,
}
#[derive(Serialize)]
pub struct Report {
    pub player_physics: PlayerPhysicsParams,
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
    physics: Res<PlayerPhysicsParams>,
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
        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(viewport.scale)),
        &tiles,
        &map,
        &viewport,
        &store,
        &server,
        &mut layouts,
        &physics,
    );
}

#[cfg(test)]
fn build_app(map: Map) -> App {
    build_app_at_scale(map, 1.0)
}
fn update_headless_root(
    viewport: Res<ScaledViewport>,
    mut roots: Query<&mut Transform, With<StageRoot>>,
) {
    if viewport.is_changed() {
        for mut transform in &mut roots {
            systems::apply_stage_root_viewport(&mut transform, Vec3::Z, viewport.scale);
        }
    }
}
pub(super) fn build_app_at_scale(map: Map, scale: f32) -> App {
    build_app_with_physics(map, scale, PlayerPhysicsParams::embedded())
}

fn build_app_with_physics(map: Map, scale: f32, physics: PlayerPhysicsParams) -> App {
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
    .add_plugins((StagePhysicsPlugin, StageGameplayPlugin))
    .insert_resource(physics)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .insert_resource(LaunchProfile::default())
    .insert_resource(ScaledViewport {
        scale,
        ..ScaledViewport::new(Vec2::new(1800.0, 1200.0))
    })
    .insert_resource(Localization::new())
    .insert_resource(GameSettings::default())
    .insert_resource(StageAudioHandles::new(Handle::default(), Handle::default()))
    .init_resource::<StageAudioState>()
    .init_resource::<ScriptEditorState>()
    .init_resource::<keystone_blocks::VplState>()
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
    .add_systems(
        Update,
        update_headless_root
            .in_set(StageSystemSet::Movement)
            .run_if(in_state(GameState::Stage)),
    )
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

/// Resolve IDs from the catalog rather than imposing a second stage-count limit.
fn scenario_map_path(scenario: &Scenario, catalog: &StageCatalog) -> Result<PathBuf> {
    let accepted = || {
        let mut ids: Vec<_> = catalog.iter().map(|stage| stage.id.0).collect();
        ids.sort_unstable();
        let range = match (ids.first(), ids.last()) {
            (Some(first), Some(last)) => format!("{first}..={last}"),
            _ => "empty".into(),
        };
        format!("{range} (catalog IDs: {ids:?})")
    };
    match (&scenario.stage_id, &scenario.ron_path) {
        (Some(id), None) if catalog.stage_by_id(StageId(*id)).is_some() => {
            Ok(PathBuf::from(format!("assets/stages/stage-{id}.ron")))
        }
        (None, Some(path)) => Ok(path.clone()),
        (Some(id), None) => Err(format!("Unknown stage_id {id}; accepted: {}", accepted()).into()),
        _ => Err(format!(
            "Specify exactly one of stage_id (accepted: {}) or ron_path",
            accepted()
        )
        .into()),
    }
}
fn load_map(scenario: &Scenario, seed: u64) -> Result<Map> {
    let path = scenario_map_path(scenario, &StageCatalog::load_from_assets())?;
    let config: ChunkGrammarConfig = ron::from_str(&fs::read_to_string(path)?)?;
    Ok(chunk_grammar_map::generate_map_with_seed(config, seed))
}

fn start_programs(world: &mut World, scenario: &Scenario) -> Result<()> {
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
/// Canvas coordinates before viewport scaling, including the map's boundary margin.
fn stage_cell_position(app: &mut App, position: Vec2) -> Vec2 {
    cell_position(app.world_mut(), position)
}
fn cell_position(world: &mut World, position: Vec2) -> Vec2 {
    let root = world
        .query_filtered::<&GlobalTransform, With<StageRoot>>()
        .single(world)
        .expect("stage root");
    let local = root
        .affine()
        .inverse()
        .transform_point3(position.extend(root.translation().z));
    let metrics = world.resource::<StageGridMetrics>();
    (local.truncate() + metrics.viewport_size * 0.5) / metrics.local_tile_size
}
fn snapshot(world: &mut World, frame: u64, digest: &mut Digest) -> Result<TrajectoryPoint> {
    let position = world
        .query_filtered::<&GlobalTransform, With<Player>>()
        .iter(world)
        .next()
        .ok_or("Player was not spawned")?
        .translation()
        .truncate();
    digest.add(&frame.to_le_bytes());
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
    let mut stones: Vec<_> = world
        .query::<(&StoneIndex, &GlobalTransform)>()
        .iter(world)
        .map(|(i, t)| (i.0, t.translation().truncate()))
        .collect();
    stones.sort_by_key(|s| s.0);
    let stones = stones
        .into_iter()
        .map(|(stone_index, p)| StoneSample {
            stone_index,
            position: position_sample(world, p),
        })
        .collect();
    let placed_tiles = tile_samples::<(
        With<systems::PlacedTile>,
        Without<crate::scenes::stage::components::DugTile>,
    )>(world);
    let dug_tiles = tile_samples::<With<crate::scenes::stage::components::DugTile>>(world);
    let cell = cell_position(world, position).floor().as_ivec2();
    Ok(TrajectoryPoint {
        stones,
        placed_tiles,
        dug_tiles,
        frame,
        cell: [cell.x, cell.y],
        world_px: [position.x, position.y],
    })
}

fn position_sample(world: &mut World, position: Vec2) -> PositionSample {
    PositionSample {
        cell: cell_position(world, position).floor().as_ivec2().to_array(),
        world_px: position.to_array(),
    }
}
fn tile_samples<F: bevy::ecs::query::QueryFilter>(world: &mut World) -> Vec<PositionSample> {
    let mut positions: Vec<_> = world
        .query_filtered::<&GlobalTransform, F>()
        .iter(world)
        .map(|t| t.translation().truncate())
        .collect();
    positions.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    positions.dedup();
    positions
        .into_iter()
        .map(|p| position_sample(world, p))
        .collect()
}

/// Identical input, VM startup and observation for offline and windowed execution.
struct SeedRun {
    scenario: Scenario,
    report: SeedReport,
    digest: Digest,
    trajectory_digest: Digest,
    inputs: Vec<InputEvent>,
    cursor: usize,
    keys: ButtonInput<KeyCode>,
}
impl SeedRun {
    fn new(scenario: Scenario, seed: u64, map: &Map) -> Result<Self> {
        let ascii = chunk_grammar_map::ascii_map(map);
        let mut digest = Digest::new();
        digest.add(&serde_json::to_vec(&scenario)?);
        digest.add(&seed.to_le_bytes());
        digest.add(ascii.as_bytes());
        let mut inputs = scenario
            .seed_inputs
            .get(&seed)
            .unwrap_or(&scenario.inputs)
            .clone();
        inputs.sort_by_key(|e| e.frame);
        Ok(Self {
            scenario,
            digest,
            trajectory_digest: Digest::new(),
            inputs,
            cursor: 0,
            keys: ButtonInput::default(),
            report: SeedReport {
                seed,
                cleared: false,
                goal_frame: None,
                frames: 0,
                termination: "max_frames".into(),
                error: None,
                terrain_hash: hash(ascii.as_bytes()),
                terrain_ascii: ascii,
                execution_hash: String::new(),
                trajectory_hash: String::new(),
                commands: vec![],
                trajectory: vec![],
            },
        })
    }
    fn before_frame(&mut self, world: &mut World) -> bool {
        let frame = self.report.frames;
        world.resource_mut::<Observation>().frame = frame;
        for change in self
            .scenario
            .scale_changes
            .iter()
            .filter(|c| c.frame == frame)
        {
            world.resource_mut::<ScaledViewport>().scale = change.scale;
        }
        if frame == self.scenario.start_frame
            && let Err(error) = start_programs(world, &self.scenario)
        {
            self.report.termination = "compile_error".into();
            self.report.error = Some(error.to_string());
            return false;
        }
        self.keys.clear();
        while let Some(event) = self.inputs.get(self.cursor).filter(|e| e.frame == frame) {
            if event.pressed {
                self.keys.press(event.key.code());
            } else {
                self.keys.release(event.key.code());
            }
            self.cursor += 1;
        }
        *world.resource_mut::<ButtonInput<KeyCode>>() = self.keys.clone();
        true
    }
    fn after_frame(&mut self, world: &mut World) -> Result<bool> {
        let frame = self.report.frames;
        self.report.frames += 1;
        let point = snapshot(world, frame, &mut self.digest)?;
        self.trajectory_digest.add(&serde_json::to_vec(&point)?);
        let editor = world.resource::<ScriptEditorState>();
        if editor.stage_cleared {
            self.report.cleared = true;
            self.report.goal_frame.get_or_insert(frame);
            self.report.termination = "goal".into();
        } else if frame >= self.scenario.start_frame && !editor.controls_enabled {
            self.report.termination = "script_error".into();
            self.report.error = world
                .resource::<Observation>()
                .runtime_error
                .lock()
                .expect("error observation lock")
                .clone()
                .or_else(|| editor.last_run_feedback.clone());
        }
        let ended = (self.report.termination != "max_frames"
            && (self.report.termination != "goal"
                || frame - self.report.goal_frame.unwrap() >= self.scenario.post_goal_frames))
            || self.report.frames == self.scenario.max_frames;
        if frame.is_multiple_of(self.scenario.trajectory_every) || ended {
            self.report.trajectory.push(point);
        }
        Ok(ended)
    }
    fn finish(mut self, world: &mut World) -> Result<SeedReport> {
        self.report.commands = std::mem::take(&mut world.resource_mut::<Observation>().commands);
        self.trajectory_digest.add(&serde_json::to_vec(&(
            &self.report.commands,
            self.report.goal_frame,
            self.report.frames,
            &self.report.termination,
            &self.report.error,
        ))?);
        self.report.trajectory_hash = self.trajectory_digest.hex();
        self.digest.add(&serde_json::to_vec(&self.report)?);
        self.report.execution_hash = self.digest.hex();
        Ok(self.report)
    }
}
fn verify_seed(scenario: &Scenario, seed: u64) -> Result<SeedReport> {
    let map = load_map(scenario, seed)?;
    let mut run = SeedRun::new(scenario.clone(), seed, &map)?;
    let mut app = build_app_with_physics(
        map,
        scenario.viewport_scale,
        scenario.player_physics.resolve()?,
    );
    while run.before_frame(app.world_mut()) {
        app.update();
        if run.after_frame(app.world_mut())? {
            break;
        }
    }
    run.finish(app.world_mut())
}
fn validate(scenario: &Scenario) -> Result<()> {
    scenario.player_physics.resolve()?;
    if !valid_scale(scenario.viewport_scale)
        || scenario
            .scale_changes
            .iter()
            .any(|c| !valid_scale(c.scale) || c.frame >= scenario.max_frames)
    {
        return Err("Viewport scales must be finite and positive; scale change frames must be below max_frames".into());
    }
    let mut scale_frames = std::collections::HashSet::new();
    if scenario
        .scale_changes
        .iter()
        .any(|c| !scale_frames.insert(c.frame))
    {
        return Err("Duplicate scale change frame".into());
    }

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
        player_physics: scenario.player_physics.resolve()?,
        schema_version: 2,
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
    let scale = profile.headless_viewport_scale.unwrap_or(1.0);
    if !valid_scale(scale) {
        return Err("--viewport-scale must be finite and positive".into());
    }
    let value = match profile.launch_type {
        LaunchType::HeadlessMeasureReach => serde_json::to_value(reach::measure(scale)?)?,
        _ => {
            let source = profile
                .headless_scenario
                .as_ref()
                .ok_or("--headless-verify <scenario.json> is required")?;
            let mut scenario: Scenario = serde_json::from_str(&fs::read_to_string(source)?)?;
            if profile.headless_viewport_scale.is_some() {
                scenario.viewport_scale = scale;
            }
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
    fn default_player_motion_matches_pre_parameterization_baselines() {
        let baseline: serde_json::Value = serde_json::from_str(
            &fs::read_to_string("examples/headless/motion-baselines.json").unwrap(),
        )
        .unwrap();
        for entry in baseline["scenarios"].as_array().unwrap() {
            let path = entry["scenario"].as_str().unwrap();
            let scenario: Scenario =
                serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
            let report = verify_seed(&scenario, 17).unwrap();
            assert_eq!(
                report.trajectory_hash,
                entry["trajectory_hash"].as_str().unwrap(),
                "{path}"
            );
            assert_eq!(report.frames, entry["frames"].as_u64().unwrap(), "{path}");
            assert_eq!(
                serde_json::to_value(report.goal_frame).unwrap(),
                entry["goal_frame"],
                "{path}"
            );
        }
    }

    #[test]
    fn trajectory_hash_is_independent_of_batch_and_sample_stride() {
        let mut scenario = scenario(2);
        let single = verify_seed(&scenario, 17).unwrap();
        scenario.seed_start = 0;
        scenario.seed_count = 1000;
        scenario.trajectory_every = 37;
        let batch = verify_seed(&scenario, 17).unwrap();
        assert_eq!(single.trajectory_hash, batch.trajectory_hash);
        assert_ne!(single.execution_hash, batch.execution_hash);
        assert!(!single.trajectory[0].stones.is_empty());
    }
    #[test]
    fn snapshot_records_placed_and_dug_tiles_in_both_coordinate_spaces() {
        let mut app = build_app(load_map(&scenario(2), 17).unwrap());
        let position = player_position(&mut app).unwrap();
        let tile = app
            .world_mut()
            .spawn((
                systems::PlacedTile,
                GlobalTransform::from_translation(position.extend(0.0)),
            ))
            .id();
        let point = snapshot(app.world_mut(), 0, &mut Digest::new()).unwrap();
        assert_eq!(point.placed_tiles.len(), 1);
        assert!(point.dug_tiles.is_empty());
        assert_eq!(point.placed_tiles[0].cell, point.cell);
        assert_eq!(point.placed_tiles[0].world_px, point.world_px);
        app.world_mut()
            .entity_mut(tile)
            .insert(crate::scenes::stage::components::DugTile {
                collider: Collider::rectangle(1.0, 1.0),
            });
        let dug = snapshot(app.world_mut(), 1, &mut Digest::new()).unwrap();
        assert!(dug.placed_tiles.is_empty());
        assert_eq!(dug.dug_tiles.len(), 1);
        assert_eq!(dug.dug_tiles[0].cell, point.cell);
        assert_eq!(dug.dug_tiles[0].world_px, point.world_px);
        app.world_mut()
            .entity_mut(tile)
            .remove::<crate::scenes::stage::components::DugTile>();
        let restored = snapshot(app.world_mut(), 2, &mut Digest::new()).unwrap();
        assert_eq!(restored.placed_tiles.len(), 1);
        assert!(restored.dug_tiles.is_empty());
    }
    #[test]
    fn graphical_and_headless_register_the_same_gameplay_and_physics() {
        use crate::scenes::stage::StageScenePlugin;
        use bevy::ecs::schedule::{NodeId, ScheduleGraph, ScheduleLabel, SystemSet};
        use bevy::ecs::system::{IntoSystem, System};
        fn reachable(graph: &ScheduleGraph, from: NodeId, to: NodeId, hierarchy: bool) -> bool {
            let edges = if hierarchy {
                graph.hierarchy().graph()
            } else {
                graph.dependency().graph()
            };
            let mut visited = std::collections::HashSet::new();
            let mut pending = vec![from];
            while let Some(node) = pending.pop() {
                if node == to {
                    return true;
                }
                if visited.insert(node) {
                    pending.extend(edges.neighbors(node));
                }
            }
            false
        }
        // The only exclusions from normal registration. Keep normal-only
        // systems in StageScenePlugin (in the original order), not in the shared
        // plugin behind conditions that silently disable them headlessly.
        let normal_only = [
            // Changes sprite frames/timers, never colliders or player motion.
            IntoSystem::into_system(systems::animate_player).system_type(),
            // Camera color grading only.
            IntoSystem::into_system(systems::update_stage_color_grading).system_type(),
            // Draws the egui editor; input is injected by the scenario driver.
            IntoSystem::into_system(systems::ui).system_type(),
            // Opens the normal-play tutorial dialog after its UI becomes ready.
            IntoSystem::into_system(systems::tick_pending_tutorial).system_type(),
            // Dismisses tutorial overlays via keyboard, absent in seeded runs.
            IntoSystem::into_system(systems::handle_tutorial_overlay_input).system_type(),
            // Normal setup loads the selected stage/UI; headless uses seeded startup setup.
            IntoSystem::into_system(systems::setup).system_type(),
            // Normal OnEnter friction is replaced by headless Startup friction after seeded setup.
            IntoSystem::into_system(
                crate::systems::engine::friction::apply_zero_friction_to_rigid_bodies,
            )
            .system_type(),
            // Normal OnExit cleanup; each headless seed drops its entire World instead.
            IntoSystem::into_system(systems::cleanup).system_type(),
            // Normal root includes window/letterbox offsets; headless uses update_headless_root.
            IntoSystem::into_system(systems::update_stage_root).system_type(),
            // Normal progression unlocks/saves stages; headless stops at the scenario boundary.
            IntoSystem::into_system(systems::advance_stage_if_cleared).system_type(),
            // Normal reload follows stage selection; headless constructs a fresh seeded World.
            IntoSystem::into_system(systems::reload_stage_if_needed).system_type(),
            // External session ownership exists only with the normal-play control service.
            IntoSystem::into_system(
                crate::resources::external_control::invalidate_external_generation,
            )
            .system_type(),
            // Offline scenario input replaces commands from the normal-play HTTP bridge.
            IntoSystem::into_system(crate::resources::external_control::drain_external_commands)
                .system_type(),
            // Offline trajectory reports replace snapshots for normal-play external clients.
            IntoSystem::into_system(crate::resources::external_control::publish_external_snapshot)
                .system_type(),
        ];
        fn registered(
            app: &mut App,
            label: bevy::ecs::schedule::InternedScheduleLabel,
        ) -> Vec<(std::any::TypeId, String)> {
            let Some(mut schedule) = app.world_mut().resource_mut::<Schedules>().remove(label)
            else {
                return vec![];
            };
            schedule.initialize(app.world_mut()).unwrap();
            let ids = schedule
                .systems()
                .unwrap()
                // Bevy inserts synchronization nodes after registration; their
                // count depends on presentation-only deferred commands too.
                .map(|(_, system)| (system.system_type(), system.name().to_string()))
                .filter(|(id, _)| *id != std::any::TypeId::of::<ApplyDeferred>())
                .collect();
            app.world_mut().resource_mut::<Schedules>().insert(schedule);
            ids
        }
        fn signature(
            app: &mut App,
            core: &[std::any::TypeId],
        ) -> (Vec<(String, String)>, Vec<(String, String)>) {
            let mut schedule = app
                .world_mut()
                .resource_mut::<Schedules>()
                .remove(Update)
                .unwrap();
            schedule.initialize(app.world_mut()).unwrap();
            let graph = schedule.graph();
            // Derive the compared systems from actual normal registration;
            // Bevy debug system names need not be enabled.
            let mut nodes = vec![];
            for id in core {
                for (index, (key, _)) in schedule
                    .systems()
                    .unwrap()
                    .filter(|(_, system)| system.system_type() == *id)
                    .enumerate()
                {
                    nodes.push((format!("{id:?}:{index}"), NodeId::System(key)));
                }
            }
            let mut membership = vec![];
            for set in [
                StageSystemSet::Input,
                StageSystemSet::Script,
                StageSystemSet::Reset,
                StageSystemSet::Animation,
                StageSystemSet::Movement,
                StageSystemSet::Collision,
                StageSystemSet::Goal,
                StageSystemSet::Progression,
                StageSystemSet::Audio,
                StageSystemSet::UI,
            ] {
                let node = NodeId::Set(graph.system_sets.get_key(set.intern()).unwrap());
                for (name, system) in &nodes {
                    if reachable(graph, node, *system, true) {
                        membership.push((name.clone(), format!("{set:?}")));
                    }
                }
                nodes.push((format!("{set:?}"), node));
            }
            let mut dependencies = vec![];
            for (an, a) in &nodes {
                for (bn, b) in &nodes {
                    if a != b && reachable(graph, *a, *b, false) {
                        dependencies.push((an.clone(), bn.clone()));
                    }
                }
            }
            membership.sort();
            dependencies.sort();
            (membership, dependencies)
        }
        let mut normal = App::new();
        normal
            .add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin))
            .init_state::<GameState>()
            .add_plugins((StagePhysicsPlugin, StageScenePlugin));
        normal.finish();
        normal.cleanup();
        let s = scenario(1);
        let mut headless = build_app(load_map(&s, 17).unwrap());
        assert!(normal.is_plugin_added::<StageGameplayPlugin>());
        assert!(headless.is_plugin_added::<StageGameplayPlugin>());
        assert!(normal.is_plugin_added::<StagePhysicsPlugin>());
        assert!(headless.is_plugin_added::<StagePhysicsPlugin>());
        let labels: Vec<_> = normal
            .world()
            .resource::<Schedules>()
            .iter()
            .map(|(_, schedule)| schedule.label())
            .collect();
        for label in labels {
            let normal_ids = registered(&mut normal, label);
            let mut headless_ids = registered(&mut headless, label);
            for (id, name) in normal_ids
                .into_iter()
                .filter(|(id, _)| !normal_only.contains(id))
            {
                let index = headless_ids
                    .iter()
                    .position(|(actual, _)| *actual == id)
                    .unwrap_or_else(|| {
                        panic!(
                            "Normal system {name} ({id:?}) missing from headless schedule {label:?}"
                        )
                    });
                headless_ids.swap_remove(index);
            }
        }
        let mut core: Vec<_> = registered(&mut normal, Update.intern())
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !normal_only.contains(id))
            .collect();
        core.sort();
        core.dedup();
        assert_eq!(
            signature(&mut normal, &core),
            signature(&mut headless, &core)
        );
        // Both entry points install StagePhysicsPlugin. Additionally compare the
        // actual fixed-step and physics/substep schedules without debug-name support.
        for label in [
            FixedPostUpdate.intern(),
            PhysicsSchedule.intern(),
            SubstepSchedule.intern(),
        ] {
            let types = |app: &mut App| {
                let mut schedule = app
                    .world_mut()
                    .resource_mut::<Schedules>()
                    .remove(label)
                    .unwrap();
                schedule.initialize(app.world_mut()).unwrap();
                schedule
                    .systems()
                    .unwrap()
                    .map(|(_, s)| s.system_type())
                    .collect::<std::collections::HashSet<_>>()
            };
            let normal_types = types(&mut normal);
            assert!(!normal_types.is_empty());
            assert_eq!(normal_types, types(&mut headless));
        }
    }

    #[test]
    fn stage_update_preserves_base_order_for_conflicting_systems() {
        use crate::scenes::stage::StageScenePlugin;
        use bevy::ecs::system::{IntoSystem, System};

        fn check(app: &mut App, graphical: bool) {
            let mut schedule = app
                .world_mut()
                .resource_mut::<Schedules>()
                .remove(Update)
                .unwrap();
            schedule.initialize(app.world_mut()).unwrap();
            let ordered: Vec<_> = schedule
                .systems()
                .unwrap()
                .map(|(_, system)| system.system_type())
                .collect();
            // These pairs were unordered conflicts in base 32a49ac. Pin the
            // executor's actual order without adding production dependencies
            // (which would themselves rebuild/reorder the schedule).
            macro_rules! before {
                ($first:path, $second:path) => {{
                    let first = IntoSystem::into_system($first).system_type();
                    let second = IntoSystem::into_system($second).system_type();
                    let a = ordered
                        .iter()
                        .position(|id| *id == first)
                        .expect(stringify!($first));
                    let b = ordered
                        .iter()
                        .position(|id| *id == second)
                        .expect(stringify!($second));
                    assert!(
                        a < b,
                        "base order changed (graphical={}): {} must precede {}",
                        graphical,
                        stringify!($first),
                        stringify!($second)
                    );
                }};
            }
            before!(
                systems::rescale_stage_motion,
                systems::reset_player_position
            );
            before!(systems::rescale_stage_motion, systems::reset_stone_position);
            before!(systems::rescale_stage_motion, systems::tick_script_program);
            before!(systems::move_player, systems::update_stone_behavior);
            if graphical {
                before!(
                    systems::handle_stone_messages,
                    crate::resources::external_control::drain_external_commands
                );
                before!(
                    systems::handle_stone_append_messages,
                    crate::resources::external_control::drain_external_commands
                );
                before!(systems::reset_stone_position, systems::update_dynamic_solid);
                before!(
                    systems::reset_player_position,
                    systems::update_dynamic_solid
                );
                before!(
                    systems::update_stone_behavior,
                    systems::update_dynamic_solid
                );
                before!(systems::update_stone_behavior, systems::update_stage_root);
                before!(systems::update_dynamic_solid, systems::update_stage_root);
            } else {
                before!(
                    systems::rescale_stage_motion,
                    systems::handle_stone_messages
                );
                before!(
                    systems::rescale_stage_motion,
                    systems::handle_stone_append_messages
                );
                before!(systems::update_dynamic_solid, systems::reset_stone_position);
                before!(
                    systems::update_dynamic_solid,
                    systems::reset_player_position
                );
                before!(systems::update_dynamic_solid, update_headless_root);
                before!(systems::update_stone_behavior, update_headless_root);
            }
        }
        let mut normal = App::new();
        normal
            .add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin))
            .init_state::<GameState>()
            .add_plugins((StagePhysicsPlugin, StageScenePlugin));
        normal.finish();
        normal.cleanup();
        check(&mut normal, true);
        let mut headless = build_app(load_map(&scenario(1), 17).unwrap());
        check(&mut headless, false);
    }

    #[test]
    fn scaled_runs_are_deterministic_and_validate_scale_changes() {
        let mut s = scenario(1);
        s.viewport_scale = 0.75;
        s.scale_changes.push(ScaleChange {
            frame: 25,
            scale: 1.5,
        });
        validate(&s).unwrap();
        assert_eq!(
            verify_seed(&s, 17).unwrap().execution_hash,
            verify_seed(&s, 17).unwrap().execution_hash
        );
        s.viewport_scale = 0.0;
        assert!(validate(&s).is_err());
        s.viewport_scale = 1.0;
        s.scale_changes[0].frame = s.max_frames;
        assert!(validate(&s).is_err());
    }

    #[test]
    fn resize_rescales_disabled_player_once_and_preserves_local_goal_descent() {
        use crate::scenes::stage::components::PlayerGoalDescent;
        for (scale, gravity_scale) in [0.5, 0.75, 1.25, 1.5, 2.0]
            .into_iter()
            .flat_map(|scale| [80.0, 88.0].map(|gravity| (scale, gravity)))
        {
            let physics = PlayerPhysicsParams {
                gravity_scale,
                ..PlayerPhysicsParams::embedded()
            };
            let mut app = build_app_with_physics(
                Map::synthetic((26, 10), vec![((5, 3), TileKind::PlayerSpawn)]),
                1.0,
                physics,
            );
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
            let world = app.world_mut();
            let player = world
                .query_filtered::<Entity, With<Player>>()
                .single(world)
                .unwrap();
            world.get_mut::<LinearVelocity>(player).unwrap().y = 100.0;
            world.resource_mut::<ScaledViewport>().scale = scale;
            assert!(!world.resource::<ScriptEditorState>().controls_enabled);
            app.update();
            assert_eq!(
                app.world().get::<LinearVelocity>(player).unwrap().y,
                100.0 * scale
            );
            assert_eq!(
                app.world().get::<GravityScale>(player).unwrap().0,
                gravity_scale * scale
            );
            app.update();
            assert_eq!(
                app.world().get::<LinearVelocity>(player).unwrap().y,
                100.0 * scale
            );

            app.world_mut().entity_mut(player).insert((
                LinearVelocity(Vec2::ZERO),
                GravityScale(0.0),
                PlayerGoalDescent {
                    target_y: -1000.0,
                    align_x: 0.0,
                    speed: 32.0,
                    original_memberships: LayerMask::ALL,
                    original_filters: LayerMask::ALL,
                    original_gravity: gravity_scale * scale,
                },
            ));
            app.world_mut().resource_mut::<ScaledViewport>().scale = 1.0;
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / 60.0,
            )));
            let before = app.world().get::<Transform>(player).unwrap().translation.y;
            app.update();
            let world = app.world();
            let descent = world.get::<PlayerGoalDescent>(player).unwrap();
            assert!((descent.original_gravity - gravity_scale).abs() < 0.0001);
            assert_eq!(world.get::<GravityScale>(player).unwrap().0, 0.0);
            assert!(
                (before - world.get::<Transform>(player).unwrap().translation.y - 32.0 / 60.0)
                    .abs()
                    < 0.001
            );
        }
    }

    #[test]
    fn resizing_mid_command_preserves_stone_local_motion() {
        use crate::util::script_types::MoveDirection;
        let run = |scale: f32| {
            let mut app = build_app(Map::synthetic(
                (26, 10),
                vec![((2, 3), TileKind::PlayerSpawn), ((10, 5), TileKind::Stone)],
            ));
            let world = app.world_mut();
            world.resource_mut::<ScriptEditorState>().controls_enabled = true;
            let stone = world
                .query_filtered::<Entity, With<StoneRune>>()
                .single(world)
                .unwrap();
            world
                .get_mut::<systems::StoneCommandState>(stone)
                .unwrap()
                .queue
                .push_back(ScriptCommand::Move(MoveDirection::Right));
            let mut positions = vec![];
            for frame in 0..90 {
                if frame == 15 {
                    app.world_mut().resource_mut::<ScaledViewport>().scale = scale;
                }
                app.update();
                positions.push(
                    app.world()
                        .get::<Transform>(stone)
                        .unwrap()
                        .translation
                        .truncate(),
                );
            }
            positions
        };
        let baseline = run(1.0);
        assert!(baseline[50].x > baseline[0].x);
        for scale in [0.5, 0.75, 1.25, 1.5, 2.0] {
            for (frame, (actual, expected)) in run(scale).iter().zip(&baseline).enumerate() {
                assert!(
                    actual.distance(*expected) < 0.01,
                    "frame={frame} scale={scale} actual={actual:?} expected={expected:?}"
                );
            }
        }
    }
    #[test]
    fn resizing_mid_command_preserves_stone_motion_above_floor() {
        use crate::util::script_types::MoveDirection;
        let run = |direction, scale: f32| {
            let mut tiles = vec![((2, 6), TileKind::PlayerSpawn), ((10, 5), TileKind::Stone)];
            tiles.extend((0..26).map(|x| ((x, 4), TileKind::Solid)));
            let mut map = Map::synthetic((26, 10), tiles);
            map.simulation_seed = Some(0);
            let mut app = build_app(map);
            let world = app.world_mut();
            world.resource_mut::<ScriptEditorState>().controls_enabled = true;
            let stone = world
                .query_filtered::<Entity, With<StoneRune>>()
                .single(world)
                .unwrap();
            let start = world
                .get::<Transform>(stone)
                .unwrap()
                .translation
                .truncate();
            world
                .get_mut::<systems::StoneCommandState>(stone)
                .unwrap()
                .queue
                .push_back(ScriptCommand::Move(direction));
            for frame in 0..90 {
                if frame == 15 {
                    app.world_mut().resource_mut::<ScaledViewport>().scale = scale;
                }
                app.update();
            }
            let world = app.world();
            assert!(
                !world
                    .get::<systems::StoneCommandState>(stone)
                    .unwrap()
                    .is_busy()
            );
            assert_eq!(world.get::<LinearVelocity>(stone).unwrap().0, Vec2::ZERO);
            (
                start,
                world
                    .get::<Transform>(stone)
                    .unwrap()
                    .translation
                    .truncate(),
            )
        };
        let mut differences = vec![];
        for direction in [MoveDirection::Right, MoveDirection::Top] {
            let (start, baseline) = run(direction, 1.0);
            assert!(
                baseline.distance(start) > 30.0,
                "baseline must complete a move"
            );
            for scale in [0.5, 1.5, 1.75, 2.0, 3.0] {
                let (_, actual) = run(direction, scale);
                let difference = actual.distance(baseline);
                eprintln!(
                    "floor resize direction={direction:?} scale={scale} baseline={baseline:?} actual={actual:?} endpoint_delta_px={difference:.9}"
                );
                differences.push((direction, scale, difference));
            }
        }
        assert!(
            differences
                .iter()
                .all(|(_, _, difference)| *difference < 1.0),
            "floor-adjacent moves must preserve their local endpoint within 1 px: {differences:?}"
        );
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
    fn scenario_stage_ids_follow_the_catalog_without_a_fixed_upper_bound() {
        let catalog = StageCatalog::load_from_assets();
        for stage in catalog.iter() {
            assert!(scenario_map_path(&scenario(stage.id.0), &catalog).is_ok());
            assert!(load_map(&scenario(stage.id.0), 17).is_ok());
        }
        let first = catalog.iter().map(|s| s.id.0).min().unwrap();
        let last = catalog.iter().map(|s| s.id.0).max().unwrap();
        for id in [first - 1, last + 1] {
            let error = scenario_map_path(&scenario(id), &catalog)
                .unwrap_err()
                .to_string();
            assert!(error.contains(&format!("{first}..={last}")), "{error}");
        }
        // Model the expanded branch by extending the loaded catalog, without
        // changing assets or coupling this test to main's current stage count.
        let mut expanded = catalog.clone();
        for id in last + 1..=20 {
            expanded
                .stages
                .push(crate::resources::stage_catalog::StageMeta {
                    id: StageId(id),
                    title: format!("Stage {id}"),
                    unlocked: true,
                });
            assert!(scenario_map_path(&scenario(id), &expanded).is_ok());
        }
        let expanded_last = expanded.iter().map(|s| s.id.0).max().unwrap();
        assert!(scenario_map_path(&scenario(expanded_last + 1), &expanded).is_err());
        // A min/max range must not accidentally admit gaps in the catalog.
        expanded.stages.retain(|s| s.id.0 != first + 1);
        assert!(scenario_map_path(&scenario(first + 1), &expanded).is_err());
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
