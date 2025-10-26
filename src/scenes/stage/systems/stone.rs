use std::collections::VecDeque;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    core::boundary::{MoveDirection, ScriptCommand},
    scenes::stage::components::StoneRune,
};

#[derive(Message, Clone)]
pub struct StoneCommandMessage {
    pub commands: Vec<ScriptCommand>,
}

#[derive(Component, Default)]
pub(crate) struct StoneCommandState {
    queue: VecDeque<ScriptCommand>,
    current: Option<StoneAction>,
}

struct MoveCommandProgress {
    direction: Vec2,
    remaining: f32,
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
const STONE_MOVE_SPEED: f32 = 80.0;
const STONE_STEP_DISTANCE: f32 = 64.0;

pub fn spawn_stone_display(
    commands: &mut Commands,
    stage_root: Entity,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
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
            Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(STONE_SCALE)),
            StoneCommandState::default(),
            RigidBody::Dynamic,
            Collider::rectangle(
                (STONE_TILE_SIZE.x as f32) * 0.5,
                (STONE_TILE_SIZE.y as f32) * 0.5,
            ),
            DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
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
        state.queue.clear();
        state.queue.extend(msg.commands.iter().cloned());
        state.current = None;
    }
}

pub fn update_stone_behavior(
    time: Res<Time>,
    mut query: Query<(&mut StoneCommandState, &mut Transform), With<StoneRune>>,
) {
    let Ok((mut state, mut transform)) = query.single_mut() else {
        return;
    };

    if state.current.is_none()
        && let Some(command) = state.queue.pop_front()
    {
        state.current = Some(match command {
            ScriptCommand::Move(direction) => StoneAction::Move(MoveCommandProgress {
                direction: direction_to_vec(direction),
                remaining: STONE_STEP_DISTANCE,
            }),
            ScriptCommand::Sleep(seconds) => {
                StoneAction::Sleep(Timer::from_seconds(seconds.max(0.0), TimerMode::Once))
            }
        });
    }

    if let Some(action) = state.current.as_mut() {
        match action {
            StoneAction::Move(progress) => {
                let direction = progress.direction;
                if direction.length_squared() <= f32::EPSILON {
                    state.current = None;
                    return;
                }

                let distance = (STONE_MOVE_SPEED * time.delta_secs()).min(progress.remaining);
                transform.translation += Vec3::new(direction.x, direction.y, 0.0) * distance;
                progress.remaining -= distance;

                if progress.remaining <= f32::EPSILON {
                    state.current = None;
                }
            }
            StoneAction::Sleep(timer) => {
                if timer.tick(time.delta()).is_finished() {
                    state.current = None;
                }
            }
        }
    }
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
