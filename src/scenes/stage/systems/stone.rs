use std::{collections::VecDeque, time::Duration};

use avian2d::prelude::*;
use bevy::prelude::*;

use super::{StageAudioHandles, StageAudioState, ui::ScriptEditorState};
use crate::{
    resources::{settings::GameSettings, tiled::TiledMapAssets},
    scenes::stage::components::{
        PlacedTile, Player, StageRoot, StageTile, StoneRune, StoneSpawnState,
    },
    util::script_types::{MoveDirection, ScriptCommand},
};

const BACKGROUND_TILE_IDS: [u32; 2] = [235, 236];

fn background_tile_id(rng: &mut rand::rngs::ThreadRng) -> u32 {
    use rand::Rng;
    let index = rng.random_range(0..(BACKGROUND_TILE_IDS.len()));
    BACKGROUND_TILE_IDS[index]
}

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
    pub step_size: f32, // Dynamic step size based on map scale
}

impl Default for StoneCommandState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            step_size: 32.0,
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
    moved_distance: f32,
    start_position: Vec3, // Position at start of move command
}

enum StoneAction {
    Move(MoveCommandProgress),
    Sleep(Timer),
    Dig(Timer, Entity),
    Place(Timer, Vec3),
}

const STONE_ATLAS_PATH: &str = "images/spr_allrunes_spritesheet_xx.png";
const STONE_TILE_SIZE: UVec2 = UVec2::new(64, 64);
const STONE_SHEET_COLUMNS: u32 = 10;
const STONE_SHEET_ROWS: u32 = 7;
const STONE_SCALE: f32 = 1.6;
pub const STONE_STEP_DISTANCE: f32 = 32.0; // 2 tiles per move
const STONE_MOVE_DURATION: f32 = 0.87;
const STONE_COLLISION_GRACE_DISTANCE: f32 = 1.0;
pub const STONE_COLLIDER_RADIUS: f32 = 16.5; // Large for player riding
pub const STONE_RAYCAST_DISTANCE: f32 = STONE_STEP_DISTANCE + 8.0 * 2.0;
const CARRY_VERTICAL_EPS: f32 = 3.0;
const CARRY_X_MARGIN: f32 = 2.0;
const STONE_ACTION_COOLDOWN: f32 = 0.2;

use crate::resources::stone_type::StoneType;

pub fn spawn_stone(
    commands: &mut Commands,
    stage_root: Entity,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    (object_x, object_y, _scale): (f32, f32, f32),
    stone_type: StoneType,
    step_size: f32,
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
        StoneType::Type3 => UVec2::new(2, 0),
        StoneType::Type4 => UVec2::new(5, 0),
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
            StoneCommandState {
                step_size,
                ..default()
            },
            StoneMotion {
                last: Vec3::new(object_x, object_y, 1.0),
                delta: Vec2::ZERO,
            },
            RigidBody::Kinematic,
            GravityScale(0.0),
            LinearVelocity(Vec2::ZERO),
            Collider::compound(vec![(
                Position::from_xy(0.0, -STONE_COLLIDER_RADIUS * 0.04),
                Rotation::degrees(0.0),
                Collider::circle(STONE_COLLIDER_RADIUS),
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
    let Some(mut state) = query.iter_mut().next() else {
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
    let Some(mut state) = query.iter_mut().next() else {
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

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_stone_behavior(
    mut commands: Commands,
    time: Res<Time>,
    audio_handles: Res<StageAudioHandles>,
    mut audio_state: ResMut<StageAudioState>,
    settings: Res<GameSettings>,
    launch_profile: Res<crate::resources::launch_profile::LaunchProfile>,
    tiled_map_assets: Res<TiledMapAssets>,
    // viewport: Res<crate::resources::design_resolution::ScaledViewport>, // Unused now
    tiles: Query<(), With<StageTile>>,
    mut gizmos: Gizmos,
    mut query: Query<
        (
            Entity,
            &mut StoneCommandState,
            &mut Transform,
            &GlobalTransform,
            &mut LinearVelocity,
            &mut StoneMotion,
            Option<&CollidingEntities>, // kept for potential future use
        ),
        With<StoneRune>,
    >,
    query_colliders: Query<&Collider>,
    spatial: SpatialQuery,
    stage_root_query: Query<Entity, With<StageRoot>>,
) {
    let Some((
        entity,
        mut state,
        mut transform,
        global_transform,
        mut velocity,
        mut motion,
        _collisions,
    )) = query.iter_mut().next()
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

        state.current = Some(match command {
            ScriptCommand::Move(direction) => {
                let dir = direction_to_vec(direction);

                // Predictive raycast: check if path is blocked before moving
                let ray_dir = Dir2::new(dir).unwrap_or(Dir2::X);
                let origin = global_transform.translation().truncate();
                let check_dist = STONE_RAYCAST_DISTANCE * global_transform.scale().x;
                let filter =
                    SpatialQueryFilter::from_mask(LayerMask::ALL).with_excluded_entities([entity]);

                let path_blocked = if let Some(hit) =
                    spatial.cast_ray(origin, ray_dir, check_dist, true, &filter)
                {
                    tiles.get(hit.entity).is_ok()
                } else {
                    false
                };

                if path_blocked {
                    // Path is blocked - skip this move, just do a tiny pause
                    info!("Move blocked by tile, skipping");
                    StoneAction::Sleep(Timer::from_seconds(0.05, TimerMode::Once))
                } else {
                    let offset = Vec3::new(dir.x, dir.y, 0.0) * state.step_size;
                    let velocity = offset.truncate() / STONE_MOVE_DURATION;
                    StoneAction::Move(MoveCommandProgress {
                        velocity,
                        timer: Timer::from_seconds(STONE_MOVE_DURATION, TimerMode::Once),
                        moved_distance: 0.0,
                        start_position: transform.translation,
                    })
                }
            }
            ScriptCommand::Sleep(seconds) => {
                StoneAction::Sleep(Timer::from_seconds(seconds.max(0.0), TimerMode::Once))
            }
            ScriptCommand::Dig(direction) => {
                let dir_vec = direction_to_vec(direction);
                let ray_dir = Dir2::new(dir_vec).unwrap_or(Dir2::X);
                let origin = global_transform.translation().truncate();
                let max_dist = STONE_RAYCAST_DISTANCE * global_transform.scale().x;

                let filter =
                    SpatialQueryFilter::from_mask(LayerMask::ALL).with_excluded_entities([entity]);

                let hit = spatial.cast_ray(origin, ray_dir, max_dist, true, &filter);

                if let Some(hit) = hit {
                    if tiles.get(hit.entity).is_ok() {
                        StoneAction::Dig(Timer::from_seconds(0.5, TimerMode::Once), hit.entity)
                    } else {
                        // Hit something else (player? wall?)
                        StoneAction::Dig(
                            Timer::from_seconds(0.5, TimerMode::Once),
                            Entity::PLACEHOLDER,
                        )
                    }
                } else {
                    // Nothing hit
                    StoneAction::Dig(
                        Timer::from_seconds(0.5, TimerMode::Once),
                        Entity::PLACEHOLDER,
                    )
                }
            }
            ScriptCommand::Place(direction) => {
                let dir_vec = direction_to_vec(direction);
                let ray_dir = Dir2::new(dir_vec).unwrap_or(Dir2::X);
                let origin = global_transform.translation().truncate();
                let max_dist = STONE_RAYCAST_DISTANCE * global_transform.scale().x;

                let filter =
                    SpatialQueryFilter::from_mask(LayerMask::ALL).with_excluded_entities([entity]);

                // Check if there is anything at the target position
                let hit = spatial.cast_ray(origin, ray_dir, max_dist, true, &filter);

                if hit.is_none() {
                    // Calculate target position for placement
                    let target_pos = transform.translation
                        + Vec3::new(dir_vec.x, dir_vec.y, 0.0) * state.step_size;
                    StoneAction::Place(Timer::from_seconds(0.5, TimerMode::Once), target_pos)
                } else {
                    // Blocked
                    StoneAction::Sleep(Timer::from_seconds(0.5, TimerMode::Once))
                }
            }
        });
    }

    let mut stop_current = false;

    // Copy step_size before mutable borrow of state.current
    let step_size = state.step_size;

    if let Some(action) = state.current.as_mut() {
        match action {
            StoneAction::Move(progress) => {
                progress.timer.tick(time.delta());
                let world_scale = global_transform.scale().x;
                progress.moved_distance +=
                    progress.velocity.length() * time.delta_secs() * world_scale;
                velocity.0 = progress.velocity * world_scale;
                // Use shape cast to check for tile in the movement direction
                // This casts a circle (same size as stone collider) to detect collisions properly
                let dir = progress.velocity.normalize_or_zero();
                let is_colliding = if dir.length_squared() > 0.0 {
                    let ray_dir = Dir2::new(dir).unwrap_or(Dir2::X);
                    let origin = global_transform.translation().truncate();
                    let check_dist = STONE_COLLIDER_RADIUS * world_scale + 2.0;
                    let filter = SpatialQueryFilter::from_mask(LayerMask::ALL)
                        .with_excluded_entities([entity]);

                    // Use shape cast with a circle matching the stone's collider
                    let cast_shape = Collider::circle(STONE_COLLIDER_RADIUS * world_scale);
                    let cast_config = ShapeCastConfig::from_max_distance(check_dist);
                    let hit = spatial.cast_shape(
                        &cast_shape,
                        origin,
                        0.0, // rotation
                        ray_dir,
                        &cast_config,
                        &filter,
                    );

                    // Debug: visualize shape cast (only when render_physics is enabled)
                    if launch_profile.render_physics {
                        let end = origin + dir * check_dist;
                        let color = if hit.is_some() {
                            Color::srgb(1.0, 0.0, 0.0)
                        } else {
                            Color::srgb(0.0, 1.0, 0.0)
                        };
                        // Draw the circle shape at origin and destination
                        gizmos.circle_2d(
                            Isometry2d::from_translation(origin),
                            STONE_COLLIDER_RADIUS * world_scale,
                            color,
                        );
                        gizmos.circle_2d(
                            Isometry2d::from_translation(end),
                            STONE_COLLIDER_RADIUS * world_scale,
                            color,
                        );
                        gizmos.line_2d(origin, end, color);
                    }

                    hit.is_some_and(|h| tiles.get(h.entity).is_ok())
                } else {
                    false
                };

                if is_colliding
                    && progress.moved_distance > STONE_COLLISION_GRACE_DISTANCE * world_scale
                {
                    info!(
                        "Collision stop: moved_distance={}, grace={}, reverting to last safe",
                        progress.moved_distance,
                        STONE_COLLISION_GRACE_DISTANCE * world_scale
                    );
                    velocity.0 = Vec2::ZERO;
                    stop_current = true;
                    // Revert to last safe position (just before collision)
                    transform.translation = progress.start_position;
                } else if !is_colliding {
                    // Update safe position to PREVIOUS frame's position
                    // (current position might already be overlapping with tile)
                    progress.start_position = prev;
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
            StoneAction::Dig(timer, entity) => {
                if timer.tick(time.delta()).is_finished() {
                    if let Ok(collider) = query_colliders.get(*entity) {
                        commands
                            .entity(*entity)
                            .remove::<Collider>()
                            .insert(Visibility::Hidden)
                            .insert(crate::scenes::stage::components::DugTile {
                                collider: collider.clone(),
                            });
                    } else {
                        // Fallback for non-colliding entities or if query fails (shouldn't happen for tiles)
                        commands.entity(*entity).despawn();
                    }
                    // Play mining sound?
                    velocity.0 = Vec2::ZERO;
                    stop_current = true;
                }
            }
            StoneAction::Place(timer, target_pos) => {
                if timer.tick(time.delta()).is_finished() {
                    // Spawn tile as child of stage_root
                    if let Some(stage_root) = stage_root_query.iter().next() {
                        // Use a Z value that ensures visibility relative to stage_root
                        // Stone is at Z=1.0. Global spawn at Z=10.0 worked.
                        // Let's try Local Z=2.0 (in front of Stone).
                        let tile_z = 2.0;
                        let spawn_pos = target_pos.truncate().extend(tile_z);

                        // Get a random background tile ID
                        let mut rng = rand::rng();
                        let tile_id = background_tile_id(&mut rng);
                        info!("Spawning placed tile id={} at {:?}", tile_id, spawn_pos);

                        let tileset = &tiled_map_assets.tileset;
                        let tile_size = tileset.tile_size();

                        // CRITICAL: Scale must match the Stone's step_size so the grid aligns visually.
                        // step_size (32.0) / tile_size (16.0) = 2.0
                        let scale = step_size / tile_size.x;

                        if let Some(tile_sprite) = tileset.atlas_sprite(tile_id) {
                            let image =
                                Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
                            let transform = Transform::from_translation(spawn_pos)
                                .with_scale(Vec3::new(scale, scale, 1.0));

                            commands.entity(stage_root).with_children(|parent| {
                                parent.spawn((
                                    StageTile,
                                    PlacedTile,
                                    image,
                                    transform,
                                    Visibility::Visible,
                                    RigidBody::Static,
                                    Collider::rectangle(tile_size.x, tile_size.y),
                                ));
                            });
                        }
                    }

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
    spatial: SpatialQuery,
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
                let query_filter = SpatialQueryFilter::from_mask(filter_mask)
                    .with_excluded_entities([p_entity, stone_entity]);

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
                    && let Some(hit) = spatial.cast_shape(
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
