use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_fluent::prelude::Localization;

use crate::{
    resources::{design_resolution::ScaledViewport, tiled::*},
    scenes::stage::components::{Goal, Player, PlayerMotion},
    util::localization::tr,
};

use super::{StageAudioHandles, StageAudioState, ui::ScriptEditorState};

const GOAL_OBJECT_ID: u32 = 194;

pub fn spawn_goal(
    commands: &mut Commands,
    stage_root: Entity,
    tiled_map_assets: &TiledMapAssets,
    viewport: &ScaledViewport,
    (object_x, object_y): (f32, f32),
) {
    let viewport_size = viewport.size;
    let tile_size = tiled_map_assets.tile_size();
    let (real_tile_size, scale) =
        tiled_map_assets.scaled_tile_size_and_scale(viewport_size, tile_size);
    info!(
        "Computed tile size!!!1: {:?}, scale: {}",
        real_tile_size, scale
    );

    commands.entity(stage_root).with_children(|parent| {
        parent.spawn((
            Goal {
                half_extents: real_tile_size * 0.5,
            },
            image_from_tileset(&tiled_map_assets.tileset, GOAL_OBJECT_ID).unwrap(),
            Transform::from_xyz(object_x, object_y, 0.0).with_scale(Vec3::splat(scale)),
            RigidBody::Static,
        ));
    });
}

fn image_from_tileset(tileset: &Tileset, id: u32) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}

type GoalCheckPlayer<'w> = (
    &'w GlobalTransform,
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
);

pub fn check_goal_completion(
    mut commands: Commands,
    mut editor_state: ResMut<ScriptEditorState>,
    mut player_query: Query<GoalCheckPlayer<'_>, With<Player>>,
    goals: Query<(&GlobalTransform, &Goal)>,
    localization: Res<Localization>,
    audio_handles: Res<StageAudioHandles>,
    mut audio_state: ResMut<StageAudioState>,
) {
    if !editor_state.controls_enabled || editor_state.stage_cleared {
        return;
    }

    let Ok((player_transform, mut velocity, mut motion)) = player_query.single_mut() else {
        return;
    };

    let player_pos = player_transform.translation().truncate();

    for (goal_transform, goal) in &goals {
        let goal_pos = goal_transform.translation().truncate();
        let delta = player_pos - goal_pos;

        if delta.x.abs() <= goal.half_extents.x && delta.y.abs() <= goal.half_extents.y {
            velocity.x = 0.0;
            velocity.y = 0.0;
            motion.is_moving = false;
            motion.is_jumping = false;

            info!("Goal reached!");
            editor_state.controls_enabled = false;
            editor_state.stage_cleared = true;
            editor_state.pending_player_reset = false;
            editor_state.last_run_feedback = Some(tr(&localization, "stage-ui-feedback-goal"));
            editor_state.stage_clear_popup_open = true;
            audio_state.play_clear_once(&mut commands, &audio_handles);
            break;
        }
    }
}
