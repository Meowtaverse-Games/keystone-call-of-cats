//! Windowed presentation around the very same seeded simulation driver.
use super::*;
use crate::{
    plugins::TiledPlugin,
    resources::{
        asset_store::{AssetGroupLoaded, LoadAssetGroup, PendingGroups},
        design_resolution::LetterboxOffsets,
        file_storage::{FileError, FileStorage, FileStorageResource},
        locale_resources::LocaleFolder,
        stage_catalog::{StageCatalog, StageId},
        stage_scripts::StageScripts,
    },
    scenes::{
        assets::{AudioKey, DEFAULT_GROUP},
        stage::StagePresentationPlugin,
    },
    systems::engine::assets_loader::{handle_load_requests, poll_pending_groups},
};
use bevy::{asset::LoadState, input::InputSystems, window::PresentMode, winit::WinitSettings};
use bevy_egui::{
    EguiContexts, EguiInput, EguiPlugin, EguiPreUpdateSet, EguiPrimaryContextPass, egui,
};
use bevy_fluent::prelude::{FluentPlugin, Locale, LocalizationBuilder};

/// UI dependencies never reach a disk/cloud storage implementation in replay.
struct NoSaveStorage;
impl FileStorage for NoSaveStorage {
    fn load(&self, _: &str) -> std::result::Result<Option<Vec<u8>>, FileError> {
        Ok(None)
    }
    fn save(&self, _: &str, _: &[u8]) -> std::result::Result<(), FileError> {
        Ok(())
    }
}
#[derive(Resource)]
struct Replay {
    run: Option<SeedRun>,
    running: bool,
    finished: bool,
    started: Instant,
    report_path: Option<PathBuf>,
    status: String,
    report: Option<Report>,
}
#[derive(Resource)]
struct Pace {
    interval: Duration,
    next: Instant,
}
fn pace(mut pace: ResMut<Pace>) {
    let now = Instant::now();
    if pace.next > now {
        std::thread::sleep(pace.next - now);
    }
    pace.next = Instant::now() + pace.interval;
}
fn simulate(replay: Res<Replay>) -> bool {
    !replay.finished
}
fn discard_manual_input(mut inputs: Query<&mut EguiInput>) {
    for mut input in &mut inputs {
        input.0.events.clear();
        input.0.hovered_files.clear();
        input.0.dropped_files.clear();
    }
}
fn begin_loading(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut writer: MessageWriter<LoadAssetGroup>,
) {
    writer.write(DEFAULT_GROUP);
    commands.insert_resource(LocaleFolder(server.load_folder("locales")));
}
#[allow(clippy::too_many_arguments)]
fn finish_loading(
    mut commands: Commands,
    mut loaded: Local<bool>,
    mut reader: MessageReader<AssetGroupLoaded>,
    server: Res<AssetServer>,
    folder: Res<LocaleFolder>,
    builder: LocalizationBuilder,
    tiles: Res<TiledMapAssets>,
    store: Res<AssetStore>,
    mut next: ResMut<NextState<GameState>>,
) {
    *loaded |= reader.read().count() > 0;
    if *loaded
        && matches!(server.get_load_state(&folder.0), Some(LoadState::Loaded))
        && tiles
            .tileset
            .image
            .as_ref()
            .is_some_and(|i| server.is_loaded_with_dependencies(&i.texture))
    {
        commands.insert_resource(builder.build(&folder.0));
        commands.insert_resource(StageAudioHandles::new(
            store.audio(AudioKey::StonePush).unwrap(),
            store.audio(AudioKey::StageClear).unwrap(),
        ));
        next.set(GameState::Stage);
    }
}
fn before_frame(world: &mut World) {
    world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
    world.resource_scope(|world, mut replay: Mut<Replay>| {
        if replay.running && !replay.run.as_mut().unwrap().before_frame(world) {
            finish(world, &mut replay);
        }
    });
}
fn finish(world: &mut World, replay: &mut Replay) {
    replay.running = false;
    replay.finished = true;
    world.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    world.resource_mut::<Time<Physics>>().pause();
    // A timeout while a stone moves must not leave its push sound looping.
    world.resource_scope(|world, mut audio: Mut<StageAudioState>| {
        audio.reset(&mut world.commands());
    });
    let result = replay.run.take().unwrap().finish(world).and_then(|seed| {
        replay.status = format!(
            "Replay finished: {} | cleared={} | frames={} | goal={:?}",
            seed.termination, seed.cleared, seed.frames, seed.goal_frame
        );
        info!(
            "{} | trajectory_hash={}",
            replay.status, seed.trajectory_hash
        );
        let seconds = replay.started.elapsed().as_secs_f64();
        let report = Report {
            schema_version: 2,
            count: 1,
            cleared: usize::from(seed.cleared),
            frames: seed.frames,
            elapsed_seconds: seconds,
            frames_per_second: seed.frames as f64 / seconds,
            seeds: vec![seed],
        };
        replay.report = Some(report);
        if let Some(path) = &replay.report_path {
            fs::write(
                path,
                serde_json::to_string_pretty(replay.report.as_ref().unwrap())? + "\n",
            )?;
        }
        Ok(())
    });
    if let Err(error) = result {
        replay.status = format!("Replay/report error: {error}");
        error!("{}", replay.status);
    }
}
fn after_frame(world: &mut World) {
    world.resource_scope(|world, mut replay: Mut<Replay>| {
        if replay.finished {
            return;
        }
        if !replay.running {
            if world
                .query_filtered::<Entity, With<Player>>()
                .iter(world)
                .next()
                .is_some()
            {
                // The first stage update has zero delta, matching headless bootstrap.
                replay.running = true;
                replay.started = Instant::now();
                world.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                    1.0 / 60.0,
                )));
            }
            return;
        }
        match replay.run.as_mut().unwrap().after_frame(world) {
            Ok(false) => (),
            Ok(true) => finish(world, &mut replay),
            Err(error) => {
                let run = replay.run.as_mut().unwrap();
                run.report.termination = "observation_error".into();
                run.report.error = Some(error.to_string());
                finish(world, &mut replay);
            }
        }
    });
}
fn result_ui(mut contexts: EguiContexts, replay: Res<Replay>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        egui::Area::new(egui::Id::new("replay-result"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Areas otherwise start narrow and wrap the status word by word.
                    let width = (ctx.content_rect().width() - 48.0).clamp(1.0, 600.0);
                    ui.set_width(width);
                    if replay.finished {
                        ui.label(&replay.status);
                        ui.label("Close the window to exit.");
                    } else if replay.running {
                        ui.label(format!(
                            "Scenario replay — frame {} (input locked)",
                            replay.run.as_ref().unwrap().report.frames
                        ));
                    } else {
                        ui.label("Loading scenario replay…");
                    }
                });
            });
    }
}
/// Fit the physical canvas into the area to the right of egui, using only
/// camera projection and translation. Coordinates here are egui logical points.
fn camera_framing(screen: Vec2, panel: f32, canvas: Vec2) -> Option<(Vec2, Vec2)> {
    if !screen.is_finite() || screen.min_element() <= 1.0 {
        return None;
    }
    let panel = panel.clamp(0.0, screen.x - 1.0);
    let margin = 16.0_f32.min((screen.x - panel) * 0.25).min(screen.y * 0.25);
    let available = Vec2::new(screen.x - panel, screen.y) - Vec2::splat(2.0 * margin);
    let units_per_point = (canvas / available).max_element();
    Some((
        screen * units_per_point,
        Vec2::new(-panel * 0.5 * units_per_point, 0.0),
    ))
}
fn frame_replay_camera(
    mut contexts: EguiContexts,
    offsets: Res<LetterboxOffsets>,
    viewport: Res<ScaledViewport>,
    mut cameras: Query<(&mut Projection, &mut Transform), With<crate::MainCamera>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let screen = ctx.content_rect().size();
    let Some((size, center)) = camera_framing(
        Vec2::new(screen.x, screen.y),
        offsets.left,
        viewport.size * viewport.scale,
    ) else {
        return;
    };
    for (mut projection, mut transform) in &mut cameras {
        if let Projection::Orthographic(ref mut projection) = *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::Fixed {
                width: size.x,
                height: size.y,
            };
            projection.scale = 1.0;
            transform.translation.x = center.x;
            transform.translation.y = center.y;
        }
    }
}

// Let the window manager account for monitor DPI, decorations and taskbars.
// A modest restore size also avoids requesting an oversized initial surface.
fn replay_window() -> Window {
    let mut window = Window {
        title: "Keystone — scenario replay".into(),
        resolution: (640, 360).into(),
        present_mode: PresentMode::AutoNoVsync,
        ..default()
    };
    window.set_maximized(true);
    window
}

pub fn run(profile: &LaunchProfile) -> Result<()> {
    let source = profile
        .headless_scenario
        .as_ref()
        .ok_or("--replay <scenario.json> is required")?;
    let scenario: Scenario = serde_json::from_str(&fs::read_to_string(source)?)?;
    validate(&scenario)?;
    let seed = profile.replay_seed.ok_or("--seed <N> is required")?;
    let speed = profile.replay_speed.unwrap_or(1.0);
    if !speed.is_finite() || !(0.01..=100.0).contains(&speed) {
        return Err("--replay-speed must be between 0.01 and 100".into());
    }
    let map = load_map(&scenario, seed)?;
    let run = SeedRun::new(scenario.clone(), seed, &map)?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: fs::canonicalize("assets")?.to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(replay_window()),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    .add_plugins((
        StagePhysicsPlugin,
        StageGameplayPlugin,
        StagePresentationPlugin,
        EguiPlugin::default(),
        FluentPlugin,
        TiledPlugin::new("images/spa.png"),
        keystone_blocks::VisualProgrammingPlugin,
        crate::scenes::audio::AudioPlugin,
    ))
    .insert_resource(WinitSettings::continuous())
    .insert_resource(Locale::new(unic_langid::langid!("en-US")))
    .insert_resource(LaunchProfile::default())
    .insert_resource(GameSettings::default())
    .insert_resource(ScriptExecutor::deterministic())
    .insert_resource(FileStorageResource::new(Arc::new(NoSaveStorage)))
    .insert_resource(ScaledViewport {
        scale: scenario.viewport_scale,
        ..ScaledViewport::new(Vec2::new(1800.0, 1200.0))
    })
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
    .insert_resource(Replay {
        run: Some(run),
        running: false,
        finished: false,
        started: Instant::now(),
        report_path: profile.headless_report.clone(),
        status: String::new(),
        report: None,
    })
    .insert_resource(Pace {
        interval: Duration::from_secs_f64(1.0 / (60.0 * speed)),
        next: Instant::now(),
    })
    .insert_resource(map)
    .init_resource::<LetterboxOffsets>()
    .init_resource::<AssetStore>()
    .init_resource::<PendingGroups>()
    .add_message::<LoadAssetGroup>()
    .add_message::<AssetGroupLoaded>()
    .init_resource::<StageScripts>()
    .init_resource::<StoneCapabilities>()
    .init_resource::<ScriptEditorState>()
    .init_resource::<StageAudioState>()
    .init_resource::<Observation>()
    .init_state::<GameState>()
    .add_systems(Startup, (crate::setup_camera, begin_loading))
    .add_systems(First, pace)
    .add_systems(
        Update,
        (handle_load_requests, poll_pending_groups, finish_loading)
            .chain()
            .run_if(in_state(GameState::Boot)),
    )
    .add_systems(
        OnEnter(GameState::Stage),
        (
            setup,
            crate::systems::engine::friction::apply_zero_friction_to_rigid_bodies.after(setup),
        ),
    )
    .add_systems(PreUpdate, before_frame.after(InputSystems))
    .add_systems(
        PreUpdate,
        discard_manual_input
            .after(EguiPreUpdateSet::ProcessInput)
            .before(EguiPreUpdateSet::BeginPass),
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
    .add_systems(
        Update,
        systems::resize_editor_buffers_system.run_if(in_state(GameState::Stage)),
    )
    .add_systems(Last, (collect_outcomes, after_frame).chain())
    .add_systems(
        PostUpdate,
        frame_replay_camera
            .after(bevy_egui::EguiPostUpdateSet::EndPass)
            .before(bevy::camera::CameraUpdateSystems)
            .before(bevy::transform::TransformSystems::Propagate),
    )
    .add_systems(
        EguiPrimaryContextPass,
        (
            crate::scenes::boot::systems::setup_font,
            result_ui.after(StageSystemSet::UI),
        ),
    );
    configure_freeze(&mut app);
    if let Some(id) = scenario.stage_id {
        let catalog = StageCatalog::load_from_assets();
        if let Some(stage) = catalog.stage_by_id(StageId(id)) {
            app.world_mut()
                .resource_mut::<systems::StageProgressionState>()
                .select_stage(stage);
        }
    }
    app.world_mut()
        .resource_mut::<avian2d::collider_tree::ColliderTreeOptimization>()
        .use_async_tasks = false;
    // Keep gameplay/physics ordering identical to the offline driver; render schedules
    // remain owned by DefaultPlugins and are not part of the simulation fingerprint.
    for (_, schedule) in app.world_mut().resource_mut::<Schedules>().iter_mut() {
        schedule.set_executor(SingleThreadedExecutor::new());
    }
    app.run();
    Ok(())
}

fn configure_freeze(app: &mut App) {
    for set in [
        StageSystemSet::Input,
        StageSystemSet::Script,
        StageSystemSet::Reset,
        StageSystemSet::Animation,
        StageSystemSet::Movement,
        StageSystemSet::Collision,
        StageSystemSet::Goal,
    ] {
        app.configure_sets(Update, set.run_if(simulate));
    }
    // Timed terrain is outside the stage sets. Freeze it alongside physics on completion.
    app.configure_sets(Update, crate::scenes::stage::TimedTerrain.run_if(simulate));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_requests_native_maximization() {
        let mut window = replay_window();
        assert_eq!(window.internal.take_maximize_request(), Some(true));
        assert_eq!(window.resolution.width(), 640.0);
        assert_eq!(window.resolution.height(), 360.0);
    }
    #[test]
    fn camera_fits_full_canvas_beside_panel_at_every_size_and_scale() {
        for screen in [
            Vec2::new(1800.0, 1200.0),
            Vec2::new(900.0, 600.0),
            Vec2::new(2400.0, 1000.0),
            // Representative monitor sizes / OS DPI scales. Native maximization
            // supplies the actual client area; framing also tolerates smaller areas.
            Vec2::new(1920.0, 1200.0) / 1.75,
            Vec2::new(2560.0, 1440.0),
            Vec2::new(1366.0, 768.0) / 1.25,
        ] {
            for fraction in [0.125, 0.25, 0.5] {
                for scale in [0.5, 0.75, 1.0, 1.25, 1.5, 2.0] {
                    let panel = screen.x * fraction;
                    let canvas = Vec2::new(1800.0, 1200.0) * scale;
                    let (view, center) = camera_framing(screen, panel, canvas).unwrap();
                    for corner in [Vec2::new(-0.5, -0.5), Vec2::new(0.5, 0.5)] {
                        let pixel = ((canvas * corner - center) / view + Vec2::splat(0.5)) * screen;
                        assert!(pixel.x >= panel && pixel.x <= screen.x);
                        assert!(pixel.y >= 0.0 && pixel.y <= screen.y);
                    }
                }
            }
        }
        assert!(camera_framing(Vec2::ZERO, 0.0, Vec2::ONE).is_none());
    }
    #[test]
    fn storage_is_always_empty_and_never_persists() {
        let storage = NoSaveStorage;
        storage.save("settings.ron", b"ignored").unwrap();
        assert_eq!(storage.load("settings.ron").unwrap(), None);
    }
    #[test]
    fn replay_schedule_matches_offline_and_freezes_at_completion() {
        for path in [
            "examples/headless/stage-1.json",
            "examples/headless/stage-2.json",
        ] {
            let scenario: Scenario =
                serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
            let expected = verify_seed(&scenario, 17).unwrap();
            let map = load_map(&scenario, 17).unwrap();
            let run = SeedRun::new(scenario.clone(), 17, &map).unwrap();
            let mut app = build_app_at_scale(map, scenario.viewport_scale);
            app.insert_resource(Replay {
                run: Some(run),
                running: true,
                finished: false,
                started: Instant::now(),
                report_path: None,
                status: String::new(),
                report: None,
            });
            app.add_systems(PreUpdate, before_frame);
            configure_freeze(&mut app);
            app.add_systems(Last, after_frame.after(collect_outcomes));
            while !app.world().resource::<Replay>().finished {
                app.update();
            }
            let actual = &app
                .world()
                .resource::<Replay>()
                .report
                .as_ref()
                .unwrap()
                .seeds[0];
            assert_eq!(actual.trajectory_hash, expected.trajectory_hash, "{path}");
            assert_eq!(
                serde_json::to_value(&actual.trajectory).unwrap(),
                serde_json::to_value(&expected.trajectory).unwrap()
            );
            let frames = actual.frames;
            let position = player_position(&mut app).unwrap();
            for _ in 0..3 {
                app.update();
            }
            assert_eq!(
                app.world()
                    .resource::<Replay>()
                    .report
                    .as_ref()
                    .unwrap()
                    .frames,
                frames
            );
            assert_eq!(player_position(&mut app).unwrap(), position);
            assert!(app.world().resource::<Time<Physics>>().is_paused());
        }
    }
}
