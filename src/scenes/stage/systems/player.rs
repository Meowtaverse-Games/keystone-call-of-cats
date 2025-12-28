use avian2d::prelude::*;
use bevy::{input::ButtonInput, prelude::*};

use crate::{
    LaunchProfile,
    resources::{asset_store::AssetStore, design_resolution::ScaledViewport},
    scenes::{assets::*, stage::components::*},
};

use super::ui::ScriptEditorState;

const PLAYER_BASE_GRAVITY_SCALE: f32 = 80.0;

pub fn spawn_player(
    commands: &mut Commands,
    stage_root: Entity,
    asset_store: &AssetStore,
    (x, y, scale): (f32, f32, f32),
    viewport_scale: f32,
) {
    let idle_frames: Vec<Handle<Image>> = PLAYER_IDLE_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();
    let run_frames: Vec<Handle<Image>> = PLAYER_RUN_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();
    let climb_frames: Vec<Handle<Image>> = PLAYER_CLIMB_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();

    let clips = PlayerAnimationClips {
        idle: idle_frames,
        run: run_frames,
        climb: climb_frames,
    };

    let initial_state = if !clips.idle.is_empty() {
        PlayerAnimationState::Idle
    } else if !clips.run.is_empty() {
        PlayerAnimationState::Run
    } else {
        PlayerAnimationState::Climb
    };

    let initial_frame = clips
        .frames(initial_state)
        .first()
        .cloned()
        .or_else(|| clips.frames(PlayerAnimationState::Run).first().cloned())
        .or_else(|| clips.frames(PlayerAnimationState::Climb).first().cloned())
        .or_else(|| clips.frames(PlayerAnimationState::Idle).first().cloned());

    let Some(initial_frame) = initial_frame else {
        warn!("Stage setup: could not determine an initial player sprite");
        return;
    };

    let player_entity = commands
        .spawn((
            Sprite::from_image(initial_frame),
            Player,
            PlayerAnimation {
                timer: Timer::from_seconds(0.12, TimerMode::Repeating),
                clips,
                state: initial_state,
                frame_index: 0,
            },
            PlayerMotion {
                speed: 120.0,
                direction: 1.0,
                is_moving: matches!(initial_state, PlayerAnimationState::Run),
                jump_speed: 380.0,
                ground_y: y,
                ..default()
            },
            PlayerSpawnState {
                translation: Vec3::new(x, y, 1.0),
                scale,
            },
            RigidBody::Dynamic,
            GravityScale(PLAYER_BASE_GRAVITY_SCALE * viewport_scale),
            LockedAxes::ROTATION_LOCKED,
            Collider::compound(vec![(
                Position::from_xy(0.0, scale * -0.6),
                Rotation::degrees(0.0),
                Collider::capsule(scale * 1.4, scale * 1.2),
            )]),
            CollidingEntities::default(),
            DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
            Transform::from_xyz(x, y, 1.0).with_scale(Vec3::splat(scale)),
        ))
        .id();
    commands.entity(stage_root).add_child(player_entity);
}

pub fn animate_player(
    time: Res<Time>,
    editor_state: Res<ScriptEditorState>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation, &PlayerMotion), With<Player>>,
) {
    for (mut sprite, mut animation, motion) in &mut query {
        if !editor_state.controls_enabled && !motion.is_climbing {
            continue;
        }

        let (desired_state, speed_multiplier) = if motion.is_moving {
            (PlayerAnimationState::Run, 2.8)
        } else if motion.is_climbing {
            (PlayerAnimationState::Climb, 1.0)
        } else {
            (PlayerAnimationState::Idle, 1.0)
        };

        if animation.state != desired_state {
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

        if animation
            .timer
            .tick(time.delta() * speed_multiplier as u32)
            .just_finished()
        {
            animation.frame_index = (animation.frame_index + 1) % frame_count;
            if let Some(handle) = animation.current_frames().get(animation.frame_index) {
                sprite.image = handle.clone();
            }
        }
    }
}

type MovePlayerComponents<'w> = (
    Entity,
    &'w GlobalTransform,
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
    &'w mut Sprite,
    &'w mut GravityScale,
    Option<&'w CollisionLayers>,
);

pub fn move_player(
    editor_state: Res<ScriptEditorState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    viewport: Res<ScaledViewport>,
    mut query: Query<MovePlayerComponents<'_>, With<Player>>,
    mut spatial_query: SpatialQuery,
    mut gizmos: Gizmos,
    launch_profile: Res<LaunchProfile>,
) {
    let Some((
        player_entity,
        player_transform,
        mut velocity,
        mut motion,
        mut sprite,
        mut gravity_scale,
        collision_layers,
    )) = query.iter_mut().next()
    else {
        return;
    };

    if !editor_state.controls_enabled {
        velocity.x = 0.0;
        motion.is_moving = false;
        sprite.flip_x = motion.direction < 0.0;
        return;
    }

    let mut input_dir: f32 = 0.0;
    let scale_factor = viewport.scale;
    let target_gravity = PLAYER_BASE_GRAVITY_SCALE * scale_factor;
    if (gravity_scale.0 - target_gravity).abs() > f32::EPSILON {
        gravity_scale.0 = target_gravity;
    }

    if keyboard_input.any_pressed(vec![KeyCode::ArrowRight, KeyCode::KeyD]) {
        input_dir += 1.0;
    }

    if keyboard_input.any_pressed(vec![KeyCode::ArrowLeft, KeyCode::KeyA]) {
        input_dir -= 1.0;
    }

    let mut desired_vx = 0.0;
    let mut facing = motion.direction;

    if input_dir.abs() > f32::EPSILON {
        let d = input_dir.signum();
        desired_vx = d * motion.speed * scale_factor;
        facing = d;
    }

    velocity.x = desired_vx;
    motion.is_moving = desired_vx.abs() > f32::EPSILON;
    motion.direction = facing;

    // Check ground contact directly via a downward shapecast from the player's feet.
    let player_transform = player_transform.compute_transform();
    let foot_origin =
        player_transform.translation + Vec3::new(0.0, -player_transform.scale.y * 12.3, 0.0);
    let cast_origin = foot_origin.truncate();
    let cast_shape = Collider::rectangle(
        player_transform.scale.x * 8.0,
        player_transform.scale.y * 0.4,
    );
    let mut cast_config = ShapeCastConfig::from_max_distance(player_transform.scale.y * 3.0);
    cast_config.ignore_origin_penetration = false;

    let filter_mask = collision_layers
        .map(|layers| layers.filters)
        .unwrap_or(LayerMask::ALL);
    let query_filter =
        SpatialQueryFilter::from_mask(filter_mask).with_excluded_entities([player_entity]);

    spatial_query.update_pipeline();
    let grounded = spatial_query
        .cast_shape_predicate(
            &cast_shape,
            cast_origin,
            0.0,
            Dir2::NEG_Y,
            &cast_config,
            &query_filter,
            &|entity| entity != player_entity,
        )
        .is_some();

    if launch_profile.render_physics {
        let cast_end = cast_origin + Vec2::new(0.0, -cast_config.max_distance);
        gizmos.line_2d(cast_origin, cast_end, Color::srgb(0.9, 0.5, 0.2));
        gizmos.rect_2d(
            Isometry2d::new(cast_origin, Rot2::default()),
            Vec2::new(player_transform.scale.x, player_transform.scale.y * 0.1),
            Color::srgb(0.2, 0.9, 0.3),
        );
    }

    if keyboard_input.any_just_pressed(vec![KeyCode::Space, KeyCode::KeyW, KeyCode::ArrowUp])
        && grounded
    {
        velocity.y = motion.jump_speed * scale_factor;
        motion.is_jumping = true;
    }

    sprite.flip_x = motion.direction < 0.0;
}

type ResetPlayerComponents<'w> = (
    &'w mut Transform,
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
    &'w mut PlayerAnimation,
    &'w mut Sprite,
    &'w PlayerSpawnState,
);

type PlayerGoalDescentComponents<'w> = (
    Entity,
    &'w mut Transform,
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
    &'w mut CollisionLayers,
    &'w mut GravityScale,
    &'w PlayerGoalDescent,
);

pub fn reset_player_position(
    mut editor_state: ResMut<ScriptEditorState>,
    mut query: Query<ResetPlayerComponents<'_>, With<Player>>,
) {
    if !editor_state.pending_player_reset {
        return;
    }

    editor_state.pending_player_reset = false;

    for (mut transform, mut velocity, mut motion, mut animation, mut sprite, spawn) in &mut query {
        transform.translation = spawn.translation;
        transform.scale = Vec3::splat(spawn.scale);

        *velocity = LinearVelocity(Vec2::ZERO);

        motion.direction = 1.0;
        motion.is_moving = false;
        motion.is_jumping = false;
        motion.ground_y = spawn.translation.y;

        animation.state = PlayerAnimationState::Idle;
        animation.frame_index = 0;
        animation.timer.reset();

        if let Some(handle) = animation.current_frames().first().cloned() {
            sprite.image = handle;
        }
        sprite.flip_x = false;
    }
}

pub fn drive_player_goal_descent(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<PlayerGoalDescentComponents<'_>, With<Player>>,
    mut count: Local<u32>,
) {
    let Some((
        entity,
        mut transform,
        mut velocity,
        mut motion,
        mut layers,
        mut gravity_scale,
        descent,
    )) = query.iter_mut().next()
    else {
        return;
    };

    layers.memberships = LayerMask::NONE;
    layers.filters = LayerMask::NONE;
    motion.is_moving = false;
    motion.is_jumping = false;
    motion.is_climbing = true;
    gravity_scale.0 = 0.0;
    velocity.x = 0.0;
    velocity.y = 0.0;

    transform.translation.x = descent.align_x;

    *count += 1;
    let descend_step = descent.speed * time.delta_secs();
    //if transform.translation.y - descend_step > descent.target_y {
    transform.translation.y -= descend_step;
    //    return;
    //}

    if *count < 280 {
        return;
    }

    *count = 0;

    transform.translation.y = descent.target_y;
    motion.is_climbing = false;
    layers.memberships = descent.original_memberships;
    layers.filters = descent.original_filters;
    gravity_scale.0 = descent.original_gravity;
    commands.entity(entity).remove::<PlayerGoalDescent>();
}
