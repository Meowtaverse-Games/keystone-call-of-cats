use avian2d::prelude::*;
use bevy::{input::ButtonInput, prelude::*};

use crate::{
    resources::asset_store::AssetStore,
    scenes::{assets::*, stage::components::*},
};

use super::ui::ScriptEditorState;

pub fn spawn_player(
    commands: &mut Commands,
    stage_root: Entity,
    asset_store: &AssetStore,
    (x, y, scale): (f32, f32, f32),
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

    commands.entity(stage_root).with_children(|parent| {
        parent.spawn((
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
                is_moving: matches!(initial_state, PlayerAnimationState::Run),
                jump_speed: 180.0,
                ground_y: y,
                ..default()
            },
            PlayerSpawnState {
                translation: Vec3::new(x, y, 1.0),
                scale,
            },
            RigidBody::Dynamic,
            GravityScale(40.0),
            LockedAxes::ROTATION_LOCKED,
            Collider::compound(vec![(
                Position::from_xy(0.0, -scale * 0.7),
                Rotation::degrees(0.0),
                Collider::capsule(scale * 1.5, scale * 1.5),
            )]),
            CollidingEntities::default(),
            DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
            Transform::from_xyz(x, y, 1.0).with_scale(Vec3::splat(scale)),
        ));
    });
}

pub fn animate_player(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation, &PlayerMotion), With<Player>>,
) {
    for (mut sprite, mut animation, motion) in &mut query {
        let desired_state = if motion.is_moving {
            PlayerAnimationState::Run
        } else if motion.is_climbing {
            PlayerAnimationState::Climb
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

type MovePlayerComponents<'w> = (
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
    &'w mut Sprite,
    Option<&'w CollidingEntities>,
);

pub fn move_player(
    editor_state: Res<ScriptEditorState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<MovePlayerComponents<'_>, With<Player>>,
) {
    let Ok((mut velocity, mut motion, mut sprite, colliding)) = query.single_mut() else {
        return;
    };

    if !editor_state.controls_enabled {
        velocity.x = 0.0;
        motion.is_moving = false;
        sprite.flip_x = motion.direction < 0.0;
        return;
    }

    let mut input_direction: f32 = 0.0;

    if keyboard_input.any_pressed(vec![KeyCode::ArrowRight, KeyCode::KeyD]) {
        input_direction += 1.0;
    }

    if keyboard_input.any_pressed(vec![KeyCode::ArrowLeft, KeyCode::KeyA]) {
        input_direction -= 1.0;
    }

    let mut desired_velocity_x = 0.0;
    let mut facing_direction = motion.direction;

    if input_direction.abs() > f32::EPSILON {
        let direction = input_direction.signum();
        desired_velocity_x = direction * motion.speed;
        facing_direction = direction;
    }

    velocity.x = desired_velocity_x;
    motion.is_moving = desired_velocity_x.abs() > f32::EPSILON;
    motion.direction = facing_direction;

    let has_contacts = colliding
        .map(|contacts| !contacts.is_empty())
        .unwrap_or(false);

    let stopped_vertically = velocity.y.abs() < 1.0;
    let grounded = has_contacts && stopped_vertically;

    if grounded && velocity.y.abs() < 0.1 {
        motion.is_jumping = false;
    } else if !grounded {
        motion.is_jumping = true;
    }

    if keyboard_input.any_just_pressed(vec![KeyCode::Space, KeyCode::KeyW, KeyCode::ArrowUp])
        && !motion.is_jumping
        && grounded
    {
        velocity.y = motion.jump_speed;
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
) {
    let Ok((
        entity,
        mut transform,
        mut velocity,
        mut motion,
        mut layers,
        mut gravity_scale,
        descent,
    )) = query.single_mut()
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

    let descend_step = descent.speed * time.delta_secs();
    if transform.translation.y - descend_step > descent.target_y {
        transform.translation.y -= descend_step;
        return;
    }

    transform.translation.y = descent.target_y;
    motion.is_climbing = false;
    layers.memberships = descent.original_memberships;
    layers.filters = descent.original_filters;
    gravity_scale.0 = descent.original_gravity;
    commands.entity(entity).remove::<PlayerGoalDescent>();
}
