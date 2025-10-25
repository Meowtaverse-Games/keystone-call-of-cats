use avian2d::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContexts, egui};

use super::components::*;
use crate::core::{
    boundary::{ScriptCommand, ScriptExecutionError, ScriptRunner},
    domain::script::ScriptExecutor,
};
use crate::scenes::assets::{PLAYER_IDLE_KEYS, PLAYER_RUN_KEYS};
use crate::{
    plugins::{TiledMapAssets, assets_loader::AssetStore, design_resolution::*},
    scenes::assets::ImageKey,
};

type StageCleanupFilter = Or<(With<StageBackground>, With<Player>, With<StageDebugMarker>)>;

#[derive(Resource, Default)]
pub struct ScriptExecutorResource(ScriptExecutor);

impl ScriptExecutorResource {
    pub fn run(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        self.0.run(source)
    }
}

#[derive(Resource, Default)]
pub struct ScriptEditorState {
    pub buffer: String,
    pub last_action: Option<EditorMenuAction>,
    pub last_run_feedback: Option<String>,
    pub last_commands: Vec<ScriptCommand>,
}

impl ScriptEditorState {
    fn apply_action(&mut self, action: EditorMenuAction) {
        // if matches!(action, EditorMenuAction::LoadExample) && self.buffer.is_empty() {}

        self.last_action = Some(action);
    }
}

fn compute_stage_root_translation(viewport: &ScaledViewport, window_size: Vec2) -> Vec3 {
    let translation = Vec2::new(
        viewport.center.x - window_size.x * 0.5,
        viewport.center.y - window_size.y * 0.5,
    );
    Vec3::new(translation.x, translation.y, 1.0)
}

fn describe_command(command: &ScriptCommand) -> String {
    match command {
        ScriptCommand::Move(direction) => format!("move({direction})"),
        ScriptCommand::Sleep(seconds) => format!("sleep({:.2}s)", seconds),
    }
}

fn summarize_commands(commands: &[ScriptCommand]) -> String {
    commands
        .iter()
        .map(describe_command)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMenuAction {
    LoadExample,
    SaveBuffer,
    RunScript,
}

impl EditorMenuAction {
    pub const ALL: [Self; 3] = [Self::LoadExample, Self::SaveBuffer, Self::RunScript];

    pub fn label(self) -> &'static str {
        match self {
            Self::LoadExample => "Load",
            Self::SaveBuffer => "Save",
            Self::RunScript => "Run",
        }
    }

    pub fn key_text(self) -> &'static str {
        match self {
            Self::LoadExample => "F1",
            Self::SaveBuffer => "F2",
            Self::RunScript => "F3",
        }
    }

    pub fn key(self) -> egui::Key {
        match self {
            Self::LoadExample => egui::Key::F1,
            Self::SaveBuffer => egui::Key::F2,
            Self::RunScript => egui::Key::F3,
        }
    }

    pub fn status_text(self) -> &'static str {
        match self {
            Self::LoadExample => "メニュー「ロード（F1）」を選択しました。",
            Self::SaveBuffer => "メニュー「セーブ（F2）」を選択しました。",
            Self::RunScript => "メニュー「実行（F3）」を選択しました。",
        }
    }
}

pub fn setup(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    tiled_map_assets: Res<TiledMapAssets>,
    viewport: Res<ScaledViewport>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let idle_frames: Vec<Handle<Image>> = PLAYER_IDLE_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();
    let run_frames: Vec<Handle<Image>> = PLAYER_RUN_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();

    if idle_frames.is_empty() && run_frames.is_empty() {
        warn!("Stage setup: no player animation frames found");
        return;
    }

    if idle_frames.is_empty() {
        warn!("Stage setup: Idle animation frames missing; falling back to run frames");
    }

    if run_frames.is_empty() {
        warn!("Stage setup: Run animation frames missing; player will stay idle");
    }

    let clips = PlayerAnimationClips {
        idle: idle_frames,
        run: run_frames,
    };

    let initial_state = if clips.idle.is_empty() {
        PlayerAnimationState::Run
    } else {
        PlayerAnimationState::Idle
    };

    let initial_frame = clips
        .frames(initial_state)
        .first()
        .cloned()
        .or_else(|| clips.frames(PlayerAnimationState::Run).first().cloned())
        .or_else(|| clips.frames(PlayerAnimationState::Idle).first().cloned());

    let Some(initial_frame) = initial_frame else {
        warn!("Stage setup: could not determine an initial player sprite");
        return;
    };

    commands.insert_resource(ScriptEditorState {
        buffer: String::from(
            "move(1)\n\
             sleep(1.0)\n\
             move(1)\n\
             sleep(1.0)\n\
            ",
        ),
        ..default()
    });

    let ground_y = -100.0;
    let x = window.resolution.width() / 2.0 * 0.25;

    commands.spawn((
        Sprite::from_image(initial_frame),
        Player,
        PlayerAnimation {
            timer: Timer::from_seconds(0.12, TimerMode::Repeating),
            clips,
            state: initial_state,
            frame_index: 0,
        },
        PlayerMotion {
            speed: 90.0,
            direction: 1.0,
            min_x: -150.0,
            max_x: 150.0,
            is_moving: matches!(initial_state, PlayerAnimationState::Run),
            vertical_velocity: 0.0,
            gravity: -600.0,
            jump_speed: 280.0,
            ground_y,
            is_jumping: false,
        },
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED,
        Collider::circle(4.5),
        DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
        Transform::from_xyz(x, 0.0, 1.0).with_scale(Vec3::splat(4.0)),
    ));

    info!("viewport: {:?}", viewport);

    let window_size = window.resolution.size();
    let stage_root_position = compute_stage_root_translation(&viewport, window_size);

    let stage_root = commands
        .spawn((
            StageRoot,
            Transform::from_translation(stage_root_position)
                .with_scale(Vec3::splat(viewport.scale)),
            Visibility::Visible,
            GlobalTransform::default(),
        ))
        .id();

    info!("viewport!!!: {:?}", viewport);

    for x in 0..10 {
        for y in 0..10 {
            let x = viewport.size.x / 2.0 / 9.0 * (x as f32);
            let y = viewport.size.y / 2.0 / 9.0 * (y as f32);

            commands.entity(stage_root).with_children(|parent| {
                parent.spawn((
                    Sprite::from_image(asset_store.image(ImageKey::Logo).unwrap()),
                    Transform::from_xyz(x, y, 0.0).with_scale(Vec3::splat(0.2)),
                ));
            });
        }
    }

    let tileset = match tiled_map_assets.tilesets().first() {
        Some(tileset) => tileset,
        None => {
            warn!("Stage setup: no tilesets available");
            return;
        }
    };

    let map_tile_dimensions = tiled_map_assets.layers().fold(UVec2::ZERO, |acc, layer| {
        let width = layer.width().max(0) as u32;
        let height = layer.height().max(0) as u32;
        UVec2::new(acc.x.max(width), acc.y.max(height))
    });

    info!("Map tile dimensions: {:?}", map_tile_dimensions,);

    let raw_tile_size = tileset
        .image()
        .map(|image| image.tile_size)
        .unwrap_or(UVec2::new(32, 32));

    info!("Tileset base tile size: {:?}", raw_tile_size,);

    let base_tile_size = Vec2::new(raw_tile_size.x.max(1) as f32, raw_tile_size.y.max(1) as f32);

    let viewport_size = viewport.size;

    let map_pixel_size = Vec2::new(
        map_tile_dimensions.x as f32 * base_tile_size.x,
        map_tile_dimensions.y as f32 * base_tile_size.y,
    );

    let scale_x = viewport_size.x / map_pixel_size.x;
    let scale_y = viewport_size.y / map_pixel_size.y;
    let scale = scale_x.min(scale_y).max(f32::EPSILON);
    let tile_size = base_tile_size * scale;

    info!(
        "Placing tiles with scale {}, tile size {:?}",
        scale, tile_size
    );

    info!("viewport: {:?}", viewport);

    commands.entity(stage_root).with_children(|parent| {
        tiled_map_assets.layers().for_each(|layer| {
            info!("Layer name: {}, type: {:?}", layer.name, layer.layer_type);
            for y in 0..layer.height() {
                for x in 0..layer.width() {
                    if let Some(tile) = layer.tile(x, y)
                        && let Some(tile_sprite) = tileset.atlas_sprite(tile.id)
                    {
                        if x == 0 {
                            info!("Spawning tile at ({}, {})", x, y);
                            info!("view port size: {}", viewport_size.x / 2.0);
                            info!(
                                "tile position: {}",
                                x as f32 * tile_size.x - viewport_size.x / 2.0
                            );
                        }

                        let tile_x = (x as f32 + 0.5) * tile_size.x - viewport_size.x / 2.0;
                        let tile_y = -((y as f32 + 0.5) * tile_size.y - viewport_size.y / 2.0);

                        if x == 0 {
                            info!("Spawning tile id {} at ({}, {})", tile.id, tile_x, tile_y);
                        }

                        let mut tile = parent.spawn((
                            StageTile,
                            Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas),
                            Transform::from_xyz(tile_x, tile_y, 0.0)
                                .with_scale(Vec3::new(scale, scale, 1.0)),
                        ));
                        if layer.name.starts_with("Ground") {
                            tile.insert((
                                RigidBody::Static,
                                Collider::rectangle(
                                    base_tile_size.x * scale,
                                    base_tile_size.y * scale,
                                ),
                                DebugRender::default().with_collider_color(
                                    Color::srgb(0.0, 1.0, 0.0).with_alpha(0.01),
                                ),
                            ));
                        };
                    }
                }
            }
        });
    });
}

pub fn cleanup(
    mut commands: Commands,
    query: Query<Entity, StageCleanupFilter>,
    tiles: Query<Entity, With<StageTile>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    for entity in &tiles {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<ScriptEditorState>();
}

pub fn animate_character(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation, &PlayerMotion), With<Player>>,
) {
    for (mut sprite, mut animation, motion) in &mut query {
        let desired_state = if motion.is_moving {
            PlayerAnimationState::Run
        } else {
            PlayerAnimationState::Idle
        };

        if animation.state != desired_state && !animation.clips.frames(desired_state).is_empty() {
            animation.state = desired_state;
            animation.frame_index = 0;
            animation.timer.reset();

            if let Some(handle) = animation.current_frames().first() {
                sprite.image = handle.clone();
            }
        }

        let frame_count = animation.current_frames().len();
        if frame_count == 0 {
            continue;
        }

        if animation.timer.tick(time.delta()).just_finished() {
            animation.frame_index = (animation.frame_index + 1) % frame_count;
            if let Some(handle) = animation.current_frames().get(animation.frame_index) {
                sprite.image = handle.clone();
            }
        }
    }
}

pub fn move_character(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut PlayerMotion, &mut Sprite), With<Player>>,
) {
    for (mut transform, mut motion, mut sprite) in &mut query {
        let mut input_direction: f32 = 0.0;

        if keyboard_input.pressed(KeyCode::ArrowRight) {
            input_direction += 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            input_direction -= 1.0;
        }

        let mut moved = false;

        if input_direction.abs() > f32::EPSILON {
            let direction = input_direction.signum();
            let delta = direction * motion.speed * time.delta_secs();
            let target_x = (transform.translation.x + delta).clamp(motion.min_x, motion.max_x);

            moved = (target_x - transform.translation.x).abs() > f32::EPSILON;
            transform.translation.x = target_x;
            motion.direction = direction;
        }

        if keyboard_input.just_pressed(KeyCode::Space) && !motion.is_jumping {
            motion.is_jumping = true;
            motion.vertical_velocity = motion.jump_speed;
        }

        if motion.is_jumping || transform.translation.y > motion.ground_y {
            motion.vertical_velocity += motion.gravity * time.delta_secs();
            transform.translation.y += motion.vertical_velocity * time.delta_secs();

            if transform.translation.y <= motion.ground_y {
                transform.translation.y = motion.ground_y;
                motion.vertical_velocity = 0.0;
                motion.is_jumping = false;
            }
        }

        motion.is_moving = moved;
        sprite.flip_x = motion.direction < 0.0;
    }
}

pub fn ui(
    mut contexts: EguiContexts,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
    mut editor: ResMut<ScriptEditorState>,
    script_executor: Res<ScriptExecutorResource>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut action_from_keys = None;
    ctx.input(|input| {
        for action in EditorMenuAction::ALL {
            if input.key_pressed(action.key()) {
                action_from_keys = Some(action);
                break;
            }
        }
    });

    let screen_width = ctx.input(|input| input.content_rect().width());

    let (min_width, max_width) = if screen_width.is_finite() && screen_width > 0.0 {
        (screen_width * 0.125, screen_width * 0.5)
    } else {
        (100.0, 300.0)
    };

    let stored_width = screen_width * 0.25;
    let default_width = if stored_width > 0.0 {
        stored_width.clamp(min_width, max_width)
    } else {
        ((min_width + max_width) * 0.5).clamp(min_width, max_width)
    };

    let left = egui::SidePanel::left("stage-left")
        .resizable(true)
        .default_width(default_width)
        .min_width(min_width)
        .max_width(max_width)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(255, 255, 255),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 150)),
            ..Default::default()
        })
        .show(ctx, |ui| {

            ui.vertical(|ui| {
                let mut pending_action = action_from_keys;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    for action in EditorMenuAction::ALL {
                        let label = format!("{} ({})", action.label(), action.key_text());
                        if ui.button(label).clicked() {
                            pending_action = Some(action);
                        }
                    }
                });

                if let Some(action) = pending_action {
                    if action == EditorMenuAction::RunScript {
                        match script_executor.run(&editor.buffer) {
                            Ok(commands) => {
                                let summary = if commands.is_empty() {
                                    "命令は返されませんでした。".to_string()
                                } else {
                                    format!(
                                        "{}件の命令: {}",
                                        commands.len(),
                                        summarize_commands(&commands)
                                    )
                                };
                                editor.last_commands = commands;
                                editor.last_run_feedback = Some(summary);
                            }
                            Err(err) => {
                                editor.last_commands.clear();
                                editor.last_run_feedback = Some(err.to_string());
                                warn!("Failed to execute script: {}", err);
                            }
                        }
                    }
                    editor.apply_action(action);
                }

                if let Some(action) = editor.last_action {
                    info!("Editor action: {:?}", action);
                    ui.label(action.status_text());
                    editor.last_action = None;
                }

                if let Some(feedback) = &editor.last_run_feedback {
                    ui.label(feedback);
                }

                if !editor.last_commands.is_empty() {
                    ui.label(format!(
                        "命令: {}",
                        summarize_commands(&editor.last_commands)
                    ));
                }

                ui.separator();

                let mut available_size = ui.available_size();
                if !available_size.x.is_finite() {
                    available_size.x = ui.max_rect().width();
                }
                if !available_size.y.is_finite() {
                    available_size.y = ui.max_rect().height();
                }

                ui.add_sized(
                    available_size,
                    egui::TextEdit::multiline(&mut editor.buffer)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            });
        })
        .response
        .rect
        .width()
        .clamp(min_width, max_width);

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
        letterbox_offsets.left = left;
    }
}

pub fn update_stage_root(
    viewport: Res<ScaledViewport>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut stage_root: Query<(&StageRoot, &mut Transform)>,
) {
    if !viewport.is_changed() {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((_root, mut transform)) = stage_root.single_mut() else {
        return;
    };

    let window_size = window.resolution.size();
    let translation = compute_stage_root_translation(&viewport, window_size);

    transform.translation = translation;
    transform.scale = Vec3::splat(viewport.scale);
}
