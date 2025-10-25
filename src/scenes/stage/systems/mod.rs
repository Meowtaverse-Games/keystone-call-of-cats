mod player;
mod stone;
mod tiles;
mod ui;

use bevy::{prelude::*, window::PrimaryWindow};

use super::components::*;
use crate::plugins::{TiledMapAssets, assets_loader::AssetStore, design_resolution::*};

pub use player::{animate_character, move_character};
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

    let player_spawn_x = 0.0;
    let player_spawn_y = window.resolution.height() / 2.0 * 0.75;
    if !player::spawn_player(
        &mut commands,
        stage_root,
        &asset_store,
        player_spawn_x,
        player_spawn_y,
    ) {
        return;
    }

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
