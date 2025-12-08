use std::{collections::VecDeque, time::Duration};

use avian2d::prelude::*;
use bevy::prelude::*;

use super::{StageAudioHandles, StageAudioState, ui::ScriptEditorState};
use crate::{
    resources::settings::GameSettings,
    scenes::stage::components::{Player, StageTile, StoneRune, StoneSpawnState},
    util::script_types::{MoveDirection, ScriptCommand},
};

#[derive(Message, Clone)]
pub struct StoneCommandMessage {
    pub commands: Vec<ScriptCommand>,
}

#[derive(Message, Clone)]
pub struct StoneAppendCommandMessage {
    pub command: ScriptCommand,
}

#[derive(Component)]
pub(crate) struct StoneCommandState {
    queue: VecDeque<ScriptCommand>,
    current: Option<StoneAction>,
    cooldown: Timer,
}

impl Default for StoneCommandState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

impl StoneCommandState {
    pub(crate) fn is_busy(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty() || !self.cooldown.is_finished()
    }
}

#[derive(Component, Default)]
pub struct StoneMotion {
    pub last: Vec3,
    pub delta: Vec2,
}

struct MoveCommandProgress {
    velocity: Vec2,
    timer: Timer,
    started_colliding: bool,
    moved_distance: f32,
}

enum StoneAction {
    Move(MoveCommandProgress),
    Sleep(Timer),
}

const STONE_ATLAS_PATH: &str = "images/spr_allrunes_spritesheet_xx.png";
const STONE_TILE_SIZE: UVec2 = UVec2::new(64, 64);
const STONE_SHEET_COLUMNS: u32 = 10;
const STONE_SHEET_ROWS: u32 = 7;
const STONE_SCALE: f32 = 1.6;
const STONE_STEP_DISTANCE: f32 = 64.0;
const STONE_MOVE_DURATION: f32 = 1.3;
const STONE_COLLISION_GRACE_DISTANCE: f32 = 1.0;
const CARRY_VERTICAL_EPS: f32 = 3.0; // 乗っているとみなす高さ誤差
const CARRY_X_MARGIN: f32 = 2.0; // 横方向の許容マージン
const STONE_ACTION_COOLDOWN: f32 = 0.2;

use crate::resources::stone_type::StoneType;

pub fn spawn_stone(
    commands: &mut Commands,
    stage_root: Entity,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    (object_x, object_y, _scale): (f32, f32, f32),
    stone_type: StoneType,
) {
    let texture = asset_server.load(STONE_ATLAS_PATH);
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        STONE_TILE_SIZE,
        STONE_SHEET_COLUMNS,
        STONE_SHEET_ROWS,
        None,
        None,
    ));

    let coord = match stone_type {
        StoneType::Type1 => UVec2::new(2, 4),
        StoneType::Type2 => UVec2::new(4, 4),
        StoneType::Type3 => UVec2::new(2, 6),
        StoneType::Type4 => UVec2::new(2, 3),
    };

    info!("stone_type: {:?}", stone_type);

    let tile_index = atlas_index(coord);
    let atlas = TextureAtlas {
        layout: layout.clone(),
        index: tile_index,
    };

    commands.entity(stage_root).with_children(|parent| {
        parent.spawn((
            StoneRune,
            Sprite::from_atlas_image(texture, atlas),
            Transform::from_xyz(object_x, object_y, 1.0).with_scale(Vec3::splat(STONE_SCALE)),
            StoneSpawnState {
                translation: Vec3::new(object_x, object_y, 1.0),
                scale: STONE_SCALE,
            },
            stone_type,
            StoneCommandState::default(),
            StoneMotion {
                last: Vec3::new(object_x, object_y, 1.0),
                delta: Vec2::ZERO,
            },
            RigidBody::Kinematic,
            GravityScale(0.0),
            LinearVelocity(Vec2::ZERO),
            Collider::compound(vec![(
                Position::from_xy(0.0, STONE_SCALE * -0.4),
                Rotation::degrees(0.0),
                Collider::circle(STONE_SCALE * 10.5),
            )]),
            LockedAxes::ROTATION_LOCKED,
            CollidingEntities::default(),
        ));
    });
}

pub fn handle_stone_messages(
    mut reader: MessageReader<StoneCommandMessage>,
    mut query: Query<&mut StoneCommandState, With<StoneRune>>,
) {
    let Ok(mut state) = query.single_mut() else {
        return;
    };

    for msg in reader.read() {
        info!("Stone received command message");
        state.queue.clear();
        state.current = None;
        state.cooldown.reset(); // Stop cooldown immediately if we force a new program?
        // Actually, let's keep it ticking normally, or finish it?
        // If we force new commands, we probably want to run them.
        state.cooldown.set_duration(Duration::ZERO);
        state.cooldown.set_elapsed(Duration::ZERO);

        state.queue.extend(msg.commands.iter().cloned());
    }
}

pub fn handle_stone_append_messages(
    mut reader: MessageReader<StoneAppendCommandMessage>,
    mut query: Query<&mut StoneCommandState, With<StoneRune>>,
) {
    let Ok(mut state) = query.single_mut() else {
        return;
    };

    for msg in reader.read() {
        state.queue.push_back(msg.command.clone());
        if matches!(msg.command, ScriptCommand::Move(_)) {
            // Move コマンドの直後に Sleep を追加することで障害物を貫通する問題を解消する
            state.queue.push_back(ScriptCommand::Sleep(0.0001));
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_stone_behavior(
    mut commands: Commands,
    time: Res<Time>,
    audio_handles: Res<StageAudioHandles>,
    mut audio_state: ResMut<StageAudioState>,
    settings: Res<GameSettings>,
    tiles: Query<(), With<StageTile>>,
    mut query: Query<
        (
            Entity,
            &mut StoneCommandState,
            &mut Transform,
            &mut LinearVelocity,
            &mut StoneMotion,
            &CollidingEntities,
        ),
        With<StoneRune>,
    >,
) {
    let Ok((_entity, mut state, mut transform, mut velocity, mut motion, collisions)) =
        query.single_mut()
    else {
        audio_state.stop_push_loop(&mut commands);
        return;
    };

    // Tick cooldown
    state.cooldown.tick(time.delta());

    // 前フレーム位置（ローカル空間）
    let prev = motion.last;

    if state.current.is_none()
        && state.cooldown.is_finished() // Only pop if cooldown is done
        && let Some(command) = state.queue.pop_front()
    {
        info!("Stone received command: {:?}", command);

        let started_colliding = collides_with_tile(collisions, &tiles);
        state.current = Some(match command {
            ScriptCommand::Move(direction) => {
                let dir = direction_to_vec(direction);
                let offset = Vec3::new(dir.x, dir.y, 0.0) * STONE_STEP_DISTANCE;
                let velocity = offset.truncate() / STONE_MOVE_DURATION / 2.0;
                StoneAction::Move(MoveCommandProgress {
                    velocity,
                    timer: Timer::from_seconds(STONE_MOVE_DURATION, TimerMode::Once),
                    started_colliding,
                    moved_distance: 0.0,
                })
            }
            ScriptCommand::Sleep(seconds) => {
                StoneAction::Sleep(Timer::from_seconds(seconds.max(0.0), TimerMode::Once))
            }
        });
    }

    let mut stop_current = false;
    let mut revert_to_prev = false;

    if let Some(action) = state.current.as_mut() {
        match action {
            StoneAction::Move(progress) => {
                progress.timer.tick(time.delta());
                progress.moved_distance += progress.velocity.length() * time.delta_secs();
                velocity.0 = progress.velocity;
                if collides_with_tile(collisions, &tiles)
                    && (!progress.started_colliding
                        || progress.moved_distance > STONE_COLLISION_GRACE_DISTANCE)
                {
                    velocity.0 = Vec2::ZERO;
                    stop_current = true;
                    // Pull back to the last safe position so the stone never stays touching a tile
                    revert_to_prev = true;
                }
                if progress.timer.is_finished() {
                    velocity.0 = Vec2::ZERO;
                    stop_current = true;
                }
            }
            StoneAction::Sleep(timer) => {
                if timer.tick(time.delta()).is_finished() {
                    velocity.0 = Vec2::ZERO;
                    stop_current = true;
                }
            }
        }
    }

    if stop_current {
        state.current = None;
        // Start cooldown
        state.cooldown = Timer::from_seconds(STONE_ACTION_COOLDOWN, TimerMode::Once);
    }

    let is_stone_moving = matches!(state.current, Some(StoneAction::Move(_)));
    if is_stone_moving {
        audio_state.ensure_push_loop(&mut commands, &audio_handles, settings.sfx_volume_linear());
    } else {
        audio_state.stop_push_loop(&mut commands);
    }

    if revert_to_prev {
        transform.translation = prev;
    }

    // このフレームの移動デルタを保存（ローカル空間の delta）
    let now = transform.translation;
    let delta = now - prev;
    motion.delta = delta.truncate();
    motion.last = now;
}

fn atlas_index(coord: UVec2) -> usize {
    (coord.y as usize) * (STONE_SHEET_COLUMNS as usize) + coord.x as usize
}

fn direction_to_vec(direction: MoveDirection) -> Vec2 {
    match direction {
        MoveDirection::Left => Vec2::NEG_X,
        MoveDirection::Right => Vec2::X,
        MoveDirection::Top => Vec2::Y,
        MoveDirection::Down => Vec2::NEG_Y,
    }
}

fn collides_with_tile(collisions: &CollidingEntities, tiles: &Query<(), With<StageTile>>) -> bool {
    collisions.iter().any(|&entity| tiles.get(entity).is_ok())
}

pub fn reset_stone_position(
    mut commands: Commands,
    editor_state: Res<ScriptEditorState>,
    mut audio_state: ResMut<StageAudioState>,
    mut query: Query<
        (
            &mut Transform,
            &mut StoneCommandState,
            &mut StoneMotion,
            &mut LinearVelocity,
            &StoneSpawnState,
        ),
        With<StoneRune>,
    >,
) {
    if !editor_state.pending_player_reset {
        return;
    }

    if let Ok((mut transform, mut state, mut motion, mut velocity, spawn)) = query.single_mut() {
        transform.translation = spawn.translation;
        transform.scale = Vec3::splat(spawn.scale);

        state.queue.clear();
        state.current = None;

        motion.delta = Vec2::ZERO;
        motion.last = spawn.translation;
        velocity.0 = Vec2::ZERO;

        audio_state.stop_push_loop(&mut commands);
    }
}

#[allow(clippy::type_complexity)]
pub fn carry_riders_with_stone(
    mut param_set: ParamSet<(
        Query<(Entity, &mut Transform, &CollisionLayers, &ColliderAabb), With<Player>>,
        Query<
            (
                Entity,
                &mut Transform,
                &mut StoneMotion,
                &mut LinearVelocity,
                &mut StoneCommandState,
            ),
            With<StoneRune>,
        >,
    )>,
    spatial_query: SpatialQuery,
) {
    // Collect moving stones first to avoid borrow conflicts (we need to mutate them later if blocked)
    // We store the data needed for the check, plus the Entity ID to look it up again for mutation.
    let moving_stones: Vec<(Entity, Vec3, Vec2)> = param_set
        .p1()
        .iter()
        .filter_map(|(entity, stone_tf, motion, _, _)| {
            if motion.delta.length_squared() <= f32::EPSILON {
                None
            } else {
                Some((entity, stone_tf.translation, motion.delta))
            }
        })
        .collect();

    if moving_stones.is_empty() {
        return;
    }

    //石の見た目サイズ（ローカル空間）から半径を計算
    let stone_half_w = STONE_TILE_SIZE.x as f32 * 0.5 * STONE_SCALE;
    let stone_half_h = STONE_TILE_SIZE.y as f32 * 0.5 * STONE_SCALE;

    // Use a separate list to track corrections relative to stone entities
    let mut stone_corrections: Vec<(Entity, Vec2)> = Vec::new();

    // Iterate players
    for (p_entity, mut p_tf, layers, aabb) in param_set.p0().iter_mut() {
        let p = p_tf.translation;

        for (stone_entity, stone_pos, delta) in moving_stones.iter() {
            let stone_entity = *stone_entity;
            let stone_pos = *stone_pos;
            let mut delta = *delta;

            // X 方向: 石の左右範囲内にいるか（マージン付き）
            let on_x = (p.x - stone_pos.x).abs() <= stone_half_w + CARRY_X_MARGIN;

            // Y 方向: プレイヤーの足元が石の天面付近にあるか
            let top_y = stone_pos.y + stone_half_h;
            let on_y = p.y >= top_y - CARRY_VERTICAL_EPS && p.y <= top_y + stone_half_h;

            if on_x && on_y {
                // Before moving, check if we would hit a wall.
                let filter_mask = layers.filters;
                let query_filter =
                    SpatialQueryFilter::from_mask(filter_mask).with_excluded_entities([p_entity]);

                // Use AABB to determine shape size.
                // AABB is in world space, but we want the shape dimensions.
                // ShapeCast shapes are local to the origin we provide? No, Collider IS the shape.
                // Collider::cuboid takes half-extents.
                let half_extents = (aabb.max - aabb.min) * 0.5;
                // We use a slightly smaller shape for the cast to avoid hitting walls we are currently touching (skin width)
                let shape =
                    Collider::rectangle(half_extents.x * 2.0 * 0.95, half_extents.y * 2.0 * 0.95);

                // Use the center of the AABB as the cast origin.
                // Note: p_tf.translation might be different from AABB center if origin is not center.
                // But AABB is updated by physics.
                let origin = aabb.center();

                let direction = delta.normalize_or_zero();
                let max_toi = delta.length();
                let mut correction = Vec2::ZERO;

                if max_toi > f32::EPSILON
                    && let Some(hit) = spatial_query.cast_shape(
                        &shape,
                        origin,
                        0.0,
                        Dir2::new(direction).unwrap_or(Dir2::X),
                        &ShapeCastConfig::from_max_distance(max_toi),
                        &query_filter,
                    )
                {
                    // If we hit something, limit the movement to the impact point
                    let allowed_dist = (hit.distance - 0.01).max(0.0);
                    let allowed_vec = direction * allowed_dist;

                    // Calculate correction: How much we STOPPED the player from moving.
                    // We want to apply this negative move to the stone.
                    // correction = allowed_vec - delta
                    correction = allowed_vec - delta;

                    delta = allowed_vec;
                }

                p_tf.translation.x += delta.x;
                p_tf.translation.y += delta.y.max(0.0);

                if correction.length_squared() > f32::EPSILON {
                    stone_corrections.push((stone_entity, correction));
                }
            }
        }
    }

    // Apply corrections to stones
    if !stone_corrections.is_empty() {
        let mut stones_query = param_set.p1();
        for (entity, correction) in stone_corrections {
            if let Ok((_, mut transform, mut motion, mut velocity, mut command_state)) =
                stones_query.get_mut(entity)
            {
                // Move stone back
                transform.translation.x += correction.x;
                transform.translation.y += correction.y;

                // Update motion.last to reflect the corrected position so next frame's delta is correct
                motion.last = transform.translation;

                // Stop the stone
                velocity.0 = Vec2::ZERO;
                command_state.current = None;
                command_state.queue.clear();
            }
        }
    }
}
