use std::collections::VecDeque;

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

#[derive(Component, Default)]
pub(crate) struct StoneCommandState {
    queue: VecDeque<ScriptCommand>,
    current: Option<StoneAction>,
}

#[derive(Component, Default)]
pub struct StoneMotion {
    pub last: Vec3,
    pub delta: Vec2,
}

struct MoveCommandProgress {
    velocity: Vec2,
    timer: Timer,
}

enum StoneAction {
    Move(MoveCommandProgress),
    Sleep(Timer),
}

const STONE_ATLAS_PATH: &str = "images/spr_allrunes_spritesheet_xx.png";
const STONE_TILE_SIZE: UVec2 = UVec2::new(64, 64);
const STONE_SHEET_COLUMNS: u32 = 10;
const STONE_SHEET_ROWS: u32 = 7;
const STONE_TILE_COORD: UVec2 = UVec2::new(2, 4);
const STONE_SCALE: f32 = 1.6;
const STONE_STEP_DISTANCE: f32 = 64.0;
const STONE_MOVE_DURATION: f32 = 1.3;
const CARRY_VERTICAL_EPS: f32 = 3.0; // 乗っているとみなす高さ誤差
const CARRY_X_MARGIN: f32 = 2.0; // 横方向の許容マージン

pub fn spawn_stone(
    commands: &mut Commands,
    stage_root: Entity,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    (object_x, object_y, _scale): (f32, f32, f32),
) {
    let texture = asset_server.load(STONE_ATLAS_PATH);
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        STONE_TILE_SIZE,
        STONE_SHEET_COLUMNS,
        STONE_SHEET_ROWS,
        None,
        None,
    ));

    let tile_index = atlas_index(STONE_TILE_COORD);
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
                Collider::circle(STONE_SCALE * 12.0),
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
        state.queue.extend(msg.commands.iter().cloned());
        state.current = None;
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
    let Ok((_entity, mut state, transform, mut velocity, mut motion, collisions)) =
        query.single_mut()
    else {
        audio_state.stop_push_loop(&mut commands);
        return;
    };
    // 前フレーム位置（ローカル空間）
    let prev = motion.last;

    if state.current.is_none()
        && let Some(command) = state.queue.pop_front()
    {
        info!("Stone received command: {:?}", command);

        state.current = Some(match command {
            ScriptCommand::Move(direction) => {
                let dir = direction_to_vec(direction);
                let offset = Vec3::new(dir.x, dir.y, 0.0) * STONE_STEP_DISTANCE;
                let velocity = offset.truncate() / STONE_MOVE_DURATION / 2.0;
                StoneAction::Move(MoveCommandProgress {
                    velocity,
                    timer: Timer::from_seconds(STONE_MOVE_DURATION, TimerMode::Once),
                })
            }
            ScriptCommand::Sleep(seconds) => {
                StoneAction::Sleep(Timer::from_seconds(seconds.max(0.0), TimerMode::Once))
            }
        });
    }

    let mut stop_current = false;

    if let Some(action) = state.current.as_mut() {
        match action {
            StoneAction::Move(progress) => {
                progress.timer.tick(time.delta());
                velocity.0 = progress.velocity;
                if collides_with_tile(collisions, &tiles) {
                    velocity.0 = Vec2::ZERO;
                    stop_current = true;
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
        Query<&mut Transform, With<Player>>,
        Query<(&Transform, &StoneMotion), With<StoneRune>>,
    )>,
) {
    // 石の見た目サイズ（ローカル空間）から半径を計算
    let stone_half_w = STONE_TILE_SIZE.x as f32 * 0.5 * STONE_SCALE;
    let stone_half_h = STONE_TILE_SIZE.y as f32 * 0.5 * STONE_SCALE;

    let moving_stones: Vec<(Vec3, Vec2)> = param_set
        .p1()
        .iter()
        .filter_map(|(stone_tf, motion)| {
            if motion.delta.length_squared() <= f32::EPSILON {
                None
            } else {
                Some((stone_tf.translation, motion.delta))
            }
        })
        .collect();

    if moving_stones.is_empty() {
        return;
    }

    for mut p_tf in param_set.p0().iter_mut() {
        let p = p_tf.translation;

        for (stone_pos, delta) in moving_stones.iter() {
            let stone_pos = *stone_pos;
            let delta = *delta;

            // X 方向: 石の左右範囲内にいるか（マージン付き）
            let on_x = (p.x - stone_pos.x).abs() <= stone_half_w + CARRY_X_MARGIN;

            // Y 方向: プレイヤーの足元が石の天面付近にあるか
            // プレイヤーの大きさが不明なため、プレイヤー中心が天面より少し上にある近傍判定に
            let top_y = stone_pos.y + stone_half_h;
            let on_y = p.y >= top_y - CARRY_VERTICAL_EPS && p.y <= top_y + stone_half_h;

            if on_x && on_y {
                p_tf.translation.x += delta.x;
                p_tf.translation.y += delta.y.max(0.0);
            }
        }
    }
}
