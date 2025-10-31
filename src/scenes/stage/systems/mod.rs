mod goal;
mod player;
mod stone;
mod tiles;
mod ui;

use bevy::{prelude::*, window::PrimaryWindow};

use super::components::*;
use crate::plugins::{TiledMapAssets, assets_loader::AssetStore, design_resolution::*};

pub use player::{PLAYER_OBJECT_ID, animate_character, move_character};
pub use stone::{
    StoneCommandMessage, carry_riders_with_stone, handle_stone_messages, update_stone_behavior,
};
use ui::ScriptEditorState;
pub use ui::ui;

type StageCleanupFilter = Or<(With<StageBackground>, With<Player>, With<StageDebugMarker>)>;

fn compute_stage_root_translation(viewport: &ScaledViewport, window_size: Vec2) -> Vec3 {
    let translation = Vec2::new(
        viewport.center.x - window_size.x * 0.5,
        viewport.center.y - window_size.y * 0.5,
    );
    Vec3::new(translation.x, translation.y, 1.0)
}

pub fn setup(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    tiled_map_assets: Res<TiledMapAssets>,
    viewport: Res<ScaledViewport>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) {
    ui::init_editor_state(&mut commands);

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

    tiles::spawn_tiles(&mut commands, stage_root, &tiled_map_assets, &viewport);

    let Some(tileset) = tiled_map_assets.tilesets().first() else {
        warn!("Stage setup: no tilesets available");
        return;
    };

    let (_, scale) =
        tiled_map_assets.scaled_tile_size_and_scale(viewport.size, tileset.tile_size());
    info!(
        "Computed player scale: {}, viewport size: {}",
        scale, viewport.size
    );

    let tile_size = tileset.tile_size();
    let viewport_size = viewport.size;
    let (real_tile_size, scale) =
        tiled_map_assets.scaled_tile_size_and_scale(viewport_size, tile_size);

    let object_layer = tiled_map_assets.object_layer();

    let Some(player_object) = object_layer.object_by_id(PLAYER_OBJECT_ID) else {
        warn!("Stage setup: no player object found in object layer");
        return;
    };

    let player_x =
        player_object.position.x * scale + real_tile_size.x / 2.0 - viewport_size.x / 2.0;
    let player_y =
        -((player_object.position.y * scale - real_tile_size.y / 2.0) - viewport_size.y / 2.0);

    player::spawn_player(
        &mut commands,
        stage_root,
        &asset_store,
        (player_x, player_y, scale),
    );

    goal::spawn_goal(&mut commands, stage_root, &tiled_map_assets, &viewport);

    stone::spawn_stone_display(&mut commands, stage_root, &asset_server, &mut atlas_layouts);
}

pub fn cleanup(
    mut commands: Commands,
    query: Query<Entity, StageCleanupFilter>,
    tiles: Query<Entity, With<StageTile>>,
    stones: Query<Entity, With<StoneRune>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    for entity in &tiles {
        commands.entity(entity).despawn();
    }

    for entity in &stones {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<ScriptEditorState>();
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
