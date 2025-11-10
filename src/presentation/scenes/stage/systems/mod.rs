mod goal;
mod player;
mod stone;
mod tiles;
mod ui;

use bevy::{ecs::system::SystemParam, prelude::*, window::PrimaryWindow};

use super::components::*;

use crate::FileStorageRes;
use crate::{
    domain::chunk_grammar_map::{self, *},
    infrastructure::engine::{
        TiledMapAssets, assets_loader::AssetStore, design_resolution::ScaledViewport,
    },
    presentation::scenes::stage::components::StageTile,
};

pub use goal::check_goal_completion;
pub use player::{animate_character, move_character, reset_player_position};
pub use stone::{
    StoneCommandMessage, carry_riders_with_stone, handle_stone_messages, update_stone_behavior,
};
use ui::ScriptEditorState;
pub use ui::ui;

const CHUNK_GRAMMAR_CONFIG_PATH: &str = "assets/chunk_grammar_map/tutorial.ron";

const MAP_SLOTS: usize = 20;

#[derive(Resource, Default)]
pub struct StageProgression {
    current_index: usize,
    pending_reload: bool,
}

impl StageProgression {
    pub fn current_map(&self) -> PlacedChunkLayout {
        new_placed_chunks()
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn advance(&mut self) -> bool {
        if self.current_index + 1 >= MAP_SLOTS {
            false
        } else {
            self.current_index += 1;
            self.pending_reload = true;
            true
        }
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= MAP_SLOTS {
            return false;
        }

        self.current_index = index;
        self.pending_reload = true;
        true
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

fn spawn_stage(
    commands: &mut Commands,
    transform: Transform,
    map_assets: &TiledMapAssets,
    placed_chunks: &PlacedChunkLayout,
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
        placed_chunks,
        viewport,
        asset_store,
        asset_server,
        atlas_layouts,
    );

    stage_root
}

fn new_placed_chunks() -> PlacedChunkLayout {
    let placed_chunks = generate_random_layout_from_file(CHUNK_GRAMMAR_CONFIG_PATH)
        .expect("failed to generate layout from config");

    chunk_grammar_map::print_ascii_map(&placed_chunks);

    placed_chunks
}

fn populate_stage_contents(
    commands: &mut Commands,
    stage_root: Entity,
    tiled_map_assets: &TiledMapAssets,
    placed_chunks: &PlacedChunkLayout,
    viewport: &ScaledViewport,
    asset_store: &AssetStore,
    asset_server: &AssetServer,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
    tiles::spawn_tiles(
        commands,
        stage_root,
        tiled_map_assets,
        placed_chunks,
        viewport,
    );

    let tile_size = Vec2::new(16.0, 16.0);
    let viewport_size = viewport.size;
    let (real_tile_size, scale) =
        tiled_map_assets.scaled_tile_size_and_scale(viewport_size, tile_size);

    let player_position = placed_chunks
        .tile_position(TileKind::PlayerSpawn)
        .unwrap_or((1, 1));

    let player_x = (player_position.0 as f32 + 1.5) * real_tile_size.x - viewport_size.x / 2.0;
    let player_y = (player_position.1 as f32 + 4.0) * real_tile_size.y - viewport_size.y / 2.0;

    player::spawn_player(
        commands,
        stage_root,
        asset_store,
        (player_x, player_y, scale),
    );

    let goal_position = placed_chunks
        .tile_position(TileKind::Goal)
        .unwrap_or((MAP_SIZE.0 - 2, MAP_SIZE.1 - 2));
    let goal_x = (goal_position.0 as f32 + 1.5) * real_tile_size.x - viewport_size.x / 2.0;
    let goal_y = (goal_position.1 as f32 + 4.0) * real_tile_size.y - viewport_size.y / 2.0;

    goal::spawn_goal(
        commands,
        stage_root,
        tiled_map_assets,
        viewport,
        (goal_x, goal_y),
    );

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
#[derive(SystemParam)]
pub struct StageSetupParams<'w, 's> {
    asset_store: Res<'w, AssetStore>,
    tiled_map_assets: Res<'w, TiledMapAssets>,
    viewport: Res<'w, ScaledViewport>,
    asset_server: Res<'w, AssetServer>,
    atlas_layouts: ResMut<'w, Assets<TextureAtlasLayout>>,
    window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    progression: ResMut<'w, StageProgression>,
    editor_state: Option<ResMut<'w, ScriptEditorState>>,
}

pub fn setup(mut commands: Commands, mut params: StageSetupParams) {
    if params.editor_state.is_none() {
        ui::init_editor_state(&mut commands);
    }

    let current_map = params.progression.current_map();

    let Ok(window) = params.window_query.single() else {
        warn!("Stage setup: primary window not available");
        return;
    };

    let stage_root_position =
        compute_stage_root_translation(params.viewport.as_ref(), window.resolution.size());
    spawn_stage(
        &mut commands,
        Transform::from_translation(stage_root_position)
            .with_scale(Vec3::splat(params.viewport.scale)),
        &params.tiled_map_assets,
        &current_map,
        params.viewport.as_ref(),
        params.asset_store.as_ref(),
        params.asset_server.as_ref(),
        params.atlas_layouts.as_mut(),
    );

    params.progression.clear_reload();
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
    mut progression: ResMut<StageProgression>,
    mut editor_state: ResMut<ScriptEditorState>,
    file_storage: Res<FileStorageRes>,
) {
    if !editor_state.stage_cleared {
        return;
    }
    use crate::application::usecase::stage_progress_usecase::StageProgressService;
    let svc = StageProgressService::new(file_storage.0.as_ref());
    if let Err(e) = svc.unlock_stage(progression.current_index()) {
        warn!("Failed to unlock stage progress: {:?}", e);
    }

    if progression.advance() {
        editor_state.last_run_feedback = Some(format!("ステージ「{}」へ進みます。", 1));
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

#[derive(SystemParam)]
pub struct StageReloadParams<'w, 's> {
    asset_store: Res<'w, AssetStore>,
    viewport: Res<'w, ScaledViewport>,
    asset_server: Res<'w, AssetServer>,
    atlas_layouts: ResMut<'w, Assets<TextureAtlasLayout>>,
    tiled_map_assets: Res<'w, TiledMapAssets>,
    window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    progression: ResMut<'w, StageProgression>,
    stage_roots: Query<'w, 's, Entity, With<StageRoot>>,
    query: Query<'w, 's, Entity, StageCleanupFilter>,
    tiles: Query<'w, 's, Entity, With<StageTile>>,
    stones: Query<'w, 's, Entity, With<StoneRune>>,
    editor_state: Option<ResMut<'w, ScriptEditorState>>,
}

pub fn reload_stage_if_needed(mut commands: Commands, mut params: StageReloadParams) {
    if !params.progression.take_pending_reload() {
        return;
    }

    let current_map = params.progression.current_map();

    cleanup_stage_entities(
        &mut commands,
        &params.stage_roots,
        &params.query,
        &params.tiles,
        &params.stones,
    );

    let Ok(window) = params.window_query.single() else {
        warn!("Stage reload: primary window not available");
        return;
    };

    let stage_root_position =
        compute_stage_root_translation(params.viewport.as_ref(), window.resolution.size());
    spawn_stage(
        &mut commands,
        Transform::from_translation(stage_root_position)
            .with_scale(Vec3::splat(params.viewport.scale)),
        &params.tiled_map_assets,
        &current_map,
        params.viewport.as_ref(),
        params.asset_store.as_ref(),
        params.asset_server.as_ref(),
        params.atlas_layouts.as_mut(),
    );

    if let Some(editor) = params.editor_state.as_deref_mut() {
        let label = format!("STAGE-{}", 1); // map_label(current_map);
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
