mod goal;
mod player;
mod stone;
mod tiles;
mod ui;

use std::path::Path;

use bevy::{prelude::*, window::PrimaryWindow};

use super::components::*;
use crate::plugins::{
    TiledMapAssets, TiledMapLibrary, assets_loader::AssetStore, design_resolution::*,
};

pub use goal::check_goal_completion;
pub use player::{PLAYER_OBJECT_ID, animate_character, move_character, reset_player_position};
pub use stone::{
    StoneCommandMessage, carry_riders_with_stone, handle_stone_messages, update_stone_behavior,
};
use ui::ScriptEditorState;
pub use ui::ui;

#[derive(Resource, Default)]
pub struct StageProgression {
    current_index: usize,
    pending_reload: bool,
}

impl StageProgression {
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn current_map<'a>(&self, library: &'a TiledMapLibrary) -> Option<&'a TiledMapAssets> {
        library.get(self.current_index)
    }

    pub fn advance(&mut self, library: &TiledMapLibrary) -> bool {
        if self.current_index + 1 >= library.len() {
            false
        } else {
            self.current_index += 1;
            self.pending_reload = true;
            true
        }
    }

    pub fn reset_if_needed(&mut self, library: &TiledMapLibrary) {
        if library.is_empty() {
            self.current_index = 0;
            self.pending_reload = false;
        } else if self.current_index >= library.len() {
            self.current_index = 0;
            self.pending_reload = true;
        }
    }

    pub fn clear_reload(&mut self) {
        self.pending_reload = false;
    }

    pub fn take_pending_reload(&mut self) -> bool {
        if self.pending_reload {
            self.pending_reload = false;
            true
        } else {
            false
        }
    }
}

type StageCleanupFilter = Or<(
    With<StageBackground>,
    With<Player>,
    With<StageDebugMarker>,
    With<Goal>,
)>;

fn compute_stage_root_translation(viewport: &ScaledViewport, window_size: Vec2) -> Vec3 {
    let translation = Vec2::new(
        viewport.center.x - window_size.x * 0.5,
        viewport.center.y - window_size.y * 0.5,
    );
    Vec3::new(translation.x, translation.y, 1.0)
}

fn map_label(map: &TiledMapAssets) -> String {
    Path::new(map.map_path())
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| map.map_path().to_string())
}

fn spawn_stage(
    commands: &mut Commands,
    transform: Transform,
    map_assets: &TiledMapAssets,
    viewport: &ScaledViewport,
    asset_store: &AssetStore,
    asset_server: &AssetServer,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
) -> Entity {
    let stage_root = commands
        .spawn((
            StageRoot,
            transform,
            Visibility::Visible,
            GlobalTransform::default(),
        ))
        .id();

    populate_stage_contents(
        commands,
        stage_root,
        map_assets,
        viewport,
        asset_store,
        asset_server,
        atlas_layouts,
    );

    stage_root
}

fn populate_stage_contents(
    commands: &mut Commands,
    stage_root: Entity,
    tiled_map_assets: &TiledMapAssets,
    viewport: &ScaledViewport,
    asset_store: &AssetStore,
    asset_server: &AssetServer,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
    tiles::spawn_tiles(commands, stage_root, tiled_map_assets, viewport);

    let Some(tileset) = tiled_map_assets.tilesets().first() else {
        warn!(
            "Stage setup: no tilesets available for map '{}'",
            tiled_map_assets.map_path()
        );
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
        warn!(
            "Stage setup: no player object found in object layer for map '{}'",
            tiled_map_assets.map_path()
        );
        return;
    };

    let player_x =
        player_object.position.x * scale + real_tile_size.x / 2.0 - viewport_size.x / 2.0;
    let player_y =
        -((player_object.position.y * scale - real_tile_size.y / 2.0) - viewport_size.y / 2.0);

    if !player::spawn_player(
        commands,
        stage_root,
        asset_store,
        (player_x, player_y, scale),
    ) {
        warn!(
            "Stage setup: failed to spawn player for map '{}'",
            tiled_map_assets.map_path()
        );
    }

    goal::spawn_goal(commands, stage_root, tiled_map_assets, viewport);

    stone::spawn_stone_display(commands, stage_root, asset_server, atlas_layouts);
}

fn cleanup_stage_entities(
    commands: &mut Commands,
    stage_roots: &Query<Entity, With<StageRoot>>,
    query: &Query<Entity, StageCleanupFilter>,
    tiles: &Query<Entity, With<StageTile>>,
    stones: &Query<Entity, With<StoneRune>>,
) {
    for entity in stage_roots.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.try_despawn();
        }
    }

    for entity in query.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.try_despawn();
        }
    }

    for entity in tiles.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.try_despawn();
        }
    }

    for entity in stones.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.try_despawn();
        }
    }
}

pub fn setup(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    tiled_maps: Res<TiledMapLibrary>,
    viewport: Res<ScaledViewport>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut progression: ResMut<StageProgression>,
    editor_state: Option<ResMut<ScriptEditorState>>,
) {
    if editor_state.is_none() {
        ui::init_editor_state(&mut commands);
    }

    if tiled_maps.is_empty() {
        warn!("Stage setup: no Tiled maps available");
        return;
    }

    progression.reset_if_needed(&tiled_maps);

    let Some(current_map) = progression.current_map(&tiled_maps) else {
        warn!(
            "Stage setup: no map available for index {}",
            progression.current_index()
        );
        return;
    };

    let Ok(window) = window_query.single() else {
        warn!("Stage setup: primary window not available");
        return;
    };

    let stage_root_position = compute_stage_root_translation(&viewport, window.resolution.size());
    spawn_stage(
        &mut commands,
        Transform::from_translation(stage_root_position).with_scale(Vec3::splat(viewport.scale)),
        current_map,
        &viewport,
        &asset_store,
        &asset_server,
        &mut atlas_layouts,
    );

    progression.clear_reload();
}

pub fn cleanup(
    mut commands: Commands,
    stage_roots: Query<Entity, With<StageRoot>>,
    query: Query<Entity, StageCleanupFilter>,
    tiles: Query<Entity, With<StageTile>>,
    stones: Query<Entity, With<StoneRune>>,
) {
    cleanup_stage_entities(&mut commands, &stage_roots, &query, &tiles, &stones);
}

pub fn advance_stage_if_cleared(
    tiled_maps: Res<TiledMapLibrary>,
    mut progression: ResMut<StageProgression>,
    mut editor_state: ResMut<ScriptEditorState>,
) {
    if !editor_state.stage_cleared {
        return;
    }

    if tiled_maps.is_empty() {
        editor_state.stage_cleared = false;
        return;
    }

    if progression.advance(&tiled_maps) {
        if let Some(next_map) = progression.current_map(&tiled_maps) {
            let label = map_label(next_map);
            editor_state.last_run_feedback = Some(format!("ステージ「{}」へ進みます。", label));
        }
        editor_state.controls_enabled = false;
        editor_state.pending_player_reset = false;
    } else {
        editor_state.controls_enabled = false;
        editor_state.pending_player_reset = false;
        editor_state
            .last_run_feedback
            .get_or_insert_with(|| "全てのステージをクリアしました！".to_string());
    }

    editor_state.stage_cleared = false;
}

pub fn reload_stage_if_needed(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    tiled_maps: Res<TiledMapLibrary>,
    viewport: Res<ScaledViewport>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut progression: ResMut<StageProgression>,
    stage_roots: Query<Entity, With<StageRoot>>,
    query: Query<Entity, StageCleanupFilter>,
    tiles: Query<Entity, With<StageTile>>,
    stones: Query<Entity, With<StoneRune>>,
    editor_state: Option<ResMut<ScriptEditorState>>,
) {
    if !progression.take_pending_reload() {
        return;
    }

    if tiled_maps.is_empty() {
        warn!("Stage reload requested but no maps are available");
        return;
    }

    let Some(current_map) = progression.current_map(&tiled_maps) else {
        warn!(
            "Stage reload: no map available for index {}",
            progression.current_index()
        );
        return;
    };

    cleanup_stage_entities(&mut commands, &stage_roots, &query, &tiles, &stones);

    let Ok(window) = window_query.single() else {
        warn!("Stage reload: primary window not available");
        return;
    };

    let stage_root_position = compute_stage_root_translation(&viewport, window.resolution.size());
    spawn_stage(
        &mut commands,
        Transform::from_translation(stage_root_position).with_scale(Vec3::splat(viewport.scale)),
        current_map,
        &viewport,
        &asset_store,
        &asset_server,
        &mut atlas_layouts,
    );

    if let Some(mut editor) = editor_state {
        let label = map_label(current_map);
        editor.controls_enabled = false;
        editor.pending_player_reset = false;
        editor.stage_cleared = false;
        editor.last_run_feedback = Some(format!("ステージ「{}」が開始されました。", label));
    }
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
