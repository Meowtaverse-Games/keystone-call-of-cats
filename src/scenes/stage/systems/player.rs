use avian2d::prelude::*;
use bevy::{input::ButtonInput, prelude::*};

use crate::{
    plugins::assets_loader::AssetStore,
    scenes::{
        assets::{PLAYER_IDLE_KEYS, PLAYER_RUN_KEYS},
        stage::components::{
            Player, PlayerAnimation, PlayerAnimationClips, PlayerAnimationState, PlayerMotion,
        },
    },
};

const PLAYER_SCALE: f32 = 4.0;
const PLAYER_GROUND_Y: f32 = -100.0;

pub fn spawn_player(
    commands: &mut Commands,
    stage_root: Entity,
    asset_store: &AssetStore,
    spawn_x: f32,
    spawn_y: f32,
) -> bool {
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
        return false;
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
        return false;
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
                jump_speed: 280.0,
                ground_y: PLAYER_GROUND_Y,
                is_jumping: false,
            },
            RigidBody::Dynamic,
            GravityScale(40.0),
            LockedAxes::ROTATION_LOCKED,
            Collider::circle(PLAYER_SCALE * 2.5),
            CollidingEntities::default(),
            DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
            Transform::from_xyz(spawn_x, spawn_y, 1.0).with_scale(Vec3::splat(PLAYER_SCALE)),
        ));
    });

    true
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

type MoveCharacterComponents<'w> = (
    &'w Transform,
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
    &'w mut Sprite,
    Option<&'w CollidingEntities>,
);

pub fn move_character(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<MoveCharacterComponents<'_>, With<Player>>,
) {
    for (transform, mut velocity, mut motion, mut sprite, colliding) in &mut query {
        let mut input_direction: f32 = 0.0;

        if keyboard_input.pressed(KeyCode::ArrowRight) {
            input_direction += 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
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

        let grounded = colliding
            .map(|contacts| !contacts.is_empty())
            .unwrap_or(false)
            || transform.translation.y <= motion.ground_y + 1.0;

        if grounded && velocity.y.abs() < 1.0 {
            motion.is_jumping = false;
        }

        if keyboard_input.just_pressed(KeyCode::Space) && !motion.is_jumping && grounded {
            velocity.y = motion.jump_speed;
            motion.is_jumping = true;
        }

        sprite.flip_x = motion.direction < 0.0;
    }
}
