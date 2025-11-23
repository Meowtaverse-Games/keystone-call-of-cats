use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_fluent::prelude::Localization;

use crate::{
    resources::{design_resolution::ScaledViewport, settings::GameSettings, tiled::*},
    scenes::stage::components::*,
    util::localization::tr,
};

use super::{StageAudioHandles, StageAudioState, ui::ScriptEditorState};

const GOAL_OBJECT_ID: u32 = 178;
const GOAL_DESCENT_SPEED: f32 = 32.0;

pub fn spawn_goal(
    commands: &mut Commands,
    stage_root: Entity,
    tiled_map_assets: &TiledMapAssets,
    viewport: &ScaledViewport,
    (object_x, object_y, scale): (f32, f32, f32),
) {
    let viewport_size = viewport.size;
    let tile_size = tiled_map_assets.tile_size();
    let (real_tile_size, _scale) =
        tiled_map_assets.scaled_tile_size_and_scale(viewport_size, tile_size);

    commands.entity(stage_root).with_children(|parent| {
        parent.spawn((
            Goal {
                half_extents: real_tile_size * 0.5,
            },
            image_from_tileset(&tiled_map_assets.tileset, GOAL_OBJECT_ID).unwrap(),
            Transform::from_xyz(object_x, object_y, 0.0).with_scale(Vec3::splat(scale)),
            RigidBody::Static,
            Collider::compound(vec![(
                Position::from_xy(0.0, scale * -0.9),
                Rotation::degrees(0.0),
                Collider::rectangle(scale * 0.3, scale * 0.3),
            )]),
            Sensor,
            CollidingEntities::default(),
        ));
    });
}

fn image_from_tileset(tileset: &Tileset, id: u32) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}

type GoalCheckPlayer<'w> = (
    Entity,
    &'w Transform,
    &'w mut LinearVelocity,
    &'w mut PlayerMotion,
    Option<&'w PlayerGoalDescent>,
    &'w mut CollisionLayers,
    &'w GravityScale,
    &'w CollidingEntities,
);

pub fn check_goal_completion(
    mut commands: Commands,
    mut editor_state: ResMut<ScriptEditorState>,
    mut player_query: Query<GoalCheckPlayer<'_>, With<Player>>,
    goals: Query<(&GlobalTransform, &Goal)>,
    tiles: Query<&GlobalTransform, With<StageTile>>,
    localization: Res<Localization>,
    audio_handles: Res<StageAudioHandles>,
    mut audio_state: ResMut<StageAudioState>,
    settings: Res<GameSettings>,
) {
    if !editor_state.controls_enabled || editor_state.stage_cleared {
        return;
    }

    let Ok((
        player_entity,
        player_transform_local,
        mut velocity,
        mut motion,
        active_descent,
        mut layers,
        gravity,
        collisions,
    )) = player_query.single_mut()
    else {
        return;
    };

    if active_descent.is_some() {
        return;
    }

    for &collider in collisions.iter() {
        let Ok((goal_transform, goal)) = goals.get(collider) else {
            continue;
        };

        velocity.x = 0.0;
        velocity.y = 0.0;
        motion.is_moving = false;
        motion.is_jumping = false;
        motion.is_climbing = true;

        info!("Goal reached!");
        editor_state.controls_enabled = false;
        editor_state.stage_cleared = true;
        editor_state.pending_player_reset = false;
        editor_state.last_run_feedback = Some(tr(&localization, "stage-ui-feedback-goal"));
        editor_state.stage_clear_popup_open = false;
        audio_state.play_clear_once(&mut commands, &audio_handles, settings.sfx_volume_linear());

        let goal_pos = goal_transform.translation().truncate();
        // Descend until the top of the bottom-most tile in view, if available; otherwise
        // fall back to the bottom of the goal collider as before.
        let target_y = tiles
            .iter()
            .map(|tf| tf.translation().y)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|min_tile_center_y| min_tile_center_y + goal.half_extents.y)
            .unwrap_or(goal_pos.y - goal.half_extents.y);
        let align_x = player_transform_local.translation.x;
        let original_memberships = layers.memberships;
        let original_filters = layers.filters;
        layers.memberships = LayerMask::NONE;
        layers.filters = LayerMask::NONE;

        commands.entity(player_entity).insert(PlayerGoalDescent {
            target_y,
            align_x,
            speed: GOAL_DESCENT_SPEED,
            original_memberships,
            original_filters,
            original_gravity: gravity.0,
        });

        break;
    }
}
