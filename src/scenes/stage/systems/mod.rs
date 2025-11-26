mod audio;
mod goal;
mod player;
mod stone;
mod tiles;
mod ui;

use bevy::{
    ecs::system::SystemParam, prelude::*, render::view::ColorGrading, window::PrimaryWindow,
};
use bevy_fluent::prelude::Localization;

use super::components::*;

use crate::{
    MainCamera,
    resources::{
        asset_store::AssetStore,
        chunk_grammar_map::{self, PlacedChunkLayout, TileKind, generate_random_layout_from_file},
        design_resolution::{LetterboxOffsets, ScaledViewport},
        stage_catalog::*,
        stage_progress::StageProgress,
        tiled::TiledMapAssets,
    },
    scenes::{assets::AudioKey, stage::components::StageTile},
    util::localization::{localized_stage_name, tr, tr_with_args},
};
use audio::{StageAudioHandles, StageAudioState};

pub use goal::check_goal_completion;
pub use player::*;
pub use stone::{
    StoneAppendCommandMessage, StoneCommandMessage, carry_riders_with_stone,
    handle_stone_append_messages, handle_stone_messages, reset_stone_position,
    update_stone_behavior,
};
use ui::{ScriptEditorState, StageTutorialOverlay};
pub use ui::{handle_tutorial_overlay_input, tick_script_program, ui};

#[derive(Resource, Default)]
pub struct StageProgressionState {
    current_stage: Option<StageMeta>,
    pending_reload: bool,
}

impl StageProgressionState {
    pub fn current_map(&self) -> PlacedChunkLayout {
        let current_stage = self.current_stage.as_ref().expect("no current stage");
        let placed_chunks = generate_random_layout_from_file(current_stage.map_path())
            .expect("failed to generate layout from config");

        for chunk in &placed_chunks {
            println!("- {}", chunk.id);
        }
        println!();

        chunk_grammar_map::print_ascii_map(&placed_chunks);

        placed_chunks
    }

    pub fn current_stage_id(&self) -> StageId {
        self.current_stage
            .as_ref()
            .map(|stage| stage.id)
            .unwrap_or_default()
    }

    pub fn current_stage(&self) -> Option<&StageMeta> {
        self.current_stage.as_ref()
    }

    pub fn advance(&mut self, stage_catalog: &StageCatalog) -> bool {
        if self.current_stage.is_none() {
            return false;
        }

        let Some(next_stage) = stage_catalog.next_stage(self.current_stage.as_ref().unwrap().id)
        else {
            return false;
        };

        self.current_stage = Some(next_stage.clone());
        self.pending_reload = true;
        true
    }

    pub fn select_stage(&mut self, stage: &StageMeta) {
        self.current_stage = Some(stage.clone());
        self.pending_reload = true;
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
    With<PlayerGroundProbe>,
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

    let player_position = placed_chunks.tile_position(TileKind::PlayerSpawn);
    info!("Spawning player at tile position {:?}", player_position);
    player::spawn_player(
        commands,
        stage_root,
        asset_store,
        tile_position_to_world(player_position, real_tile_size, viewport_size, scale, 2.2),
        viewport.scale,
    );

    let stone_position = placed_chunks.tile_position(TileKind::Stone);
    stone::spawn_stone(
        commands,
        stage_root,
        asset_server,
        atlas_layouts,
        tile_position_to_world(stone_position, real_tile_size, viewport_size, scale, 0.0),
    );

    placed_chunks
        .tile_positions(TileKind::Goal)
        .iter()
        .for_each(|&goal_position| {
            goal::spawn_goal(
                commands,
                stage_root,
                tiled_map_assets,
                viewport,
                tile_position_to_world(goal_position, real_tile_size, viewport_size, scale, 2.0),
            );
        });
}

fn tile_position_to_world(
    tile_pos: (isize, isize),
    tile_size: Vec2,
    viewport_size: Vec2,
    scale: f32,
    adjust_y: f32,
) -> (f32, f32, f32) {
    (
        (tile_pos.0 as f32 + 0.5) * tile_size.x - viewport_size.x / 2.0,
        (tile_pos.1 as f32 + 0.5) * tile_size.y - adjust_y - viewport_size.y / 2.0,
        scale,
    )
}

fn cleanup_stage_entities(
    commands: &mut Commands,
    stage_roots: &Query<Entity, With<StageRoot>>,
    query: &Query<Entity, StageCleanupFilter>,
    tiles: &Query<Entity, With<StageTile>>,
    stones: &Query<Entity, With<StoneRune>>,
    tutorial_overlays: &Query<Entity, With<StageTutorialOverlay>>,
) {
    for entity in stage_roots.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            // Remove the entire stage subtree to avoid leaving behind child sensors/colliders.
            entity_cmd.despawn();
        }
    }

    for entity in query.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.despawn();
        }
    }

    for entity in tiles.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.despawn();
        }
    }

    for entity in stones.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.despawn();
        }
    }

    for entity in tutorial_overlays.iter() {
        if let Ok(mut entity_cmd) = commands.get_entity(entity) {
            entity_cmd.despawn();
        }
    }
}
#[derive(SystemParam)]
pub struct StageSetupParams<'w, 's> {
    asset_store: Res<'w, AssetStore>,
    tiled_map_assets: Res<'w, TiledMapAssets>,
    viewport: Res<'w, ScaledViewport>,
    letterbox_offsets: Res<'w, LetterboxOffsets>,
    asset_server: Res<'w, AssetServer>,
    atlas_layouts: ResMut<'w, Assets<TextureAtlasLayout>>,
    window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    progression: ResMut<'w, StageProgressionState>,
    editor_state: Option<ResMut<'w, ScriptEditorState>>,
    audio_handles: Option<Res<'w, StageAudioHandles>>,
    audio_state: Option<ResMut<'w, StageAudioState>>,
    localization: Res<'w, Localization>,
}

pub fn setup(mut commands: Commands, mut params: StageSetupParams) {
    let current_stage_id = params.progression.current_stage_id();
    match params.editor_state.as_deref_mut() {
        Some(editor) => {
            editor.set_tutorial_for_stage(current_stage_id);
            editor.set_command_help_for_stage(current_stage_id);
            editor.controls_enabled = false;
            editor.pending_player_reset = false;
            editor.stage_cleared = false;
            editor.stage_clear_popup_open = false;
            editor.active_program = None;
        }
        None => ui::init_editor_state(&mut commands, current_stage_id),
    }

    if params.audio_handles.is_none() {
        match (
            params.asset_store.audio(AudioKey::StonePush),
            params.asset_store.audio(AudioKey::StageClear),
        ) {
            (Some(stone_move), Some(stage_clear)) => {
                commands.insert_resource(StageAudioHandles::new(stone_move, stage_clear));
            }
            _ => warn!("Stage audio handles are not available in the asset store"),
        }
    }
    if let Some(audio_state) = params.audio_state.as_deref_mut() {
        audio_state.reset(&mut commands);
    } else {
        commands.insert_resource(StageAudioState::default());
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

    let tutorial_dialog = params
        .editor_state
        .as_ref()
        .and_then(|editor| editor.tutorial_dialog.clone())
        .or_else(|| ui::tutorial_dialog_for_stage(current_stage_id));
    if let Some(dialog) = tutorial_dialog {
        ui::spawn_tutorial_overlay(
            &mut commands,
            params.asset_store.as_ref(),
            params.localization.as_ref(),
            &dialog,
            params.letterbox_offsets.as_ref(),
        );
    }

    params.progression.clear_reload();
}

pub fn cleanup(
    mut commands: Commands,
    stage_roots: Query<Entity, With<StageRoot>>,
    query: Query<Entity, StageCleanupFilter>,
    tiles: Query<Entity, With<StageTile>>,
    stones: Query<Entity, With<StoneRune>>,
    tutorial_overlays: Query<Entity, With<StageTutorialOverlay>>,
    audio_state: Option<ResMut<StageAudioState>>,
) {
    if let Some(mut audio_state) = audio_state {
        audio_state.stop_push_loop(&mut commands);
    }
    cleanup_stage_entities(
        &mut commands,
        &stage_roots,
        &query,
        &tiles,
        &stones,
        &tutorial_overlays,
    );
    commands.remove_resource::<StageAudioState>();
}

pub fn advance_stage_if_cleared(
    mut progression: ResMut<StageProgressionState>,
    mut editor_state: ResMut<ScriptEditorState>,
    mut progress: ResMut<StageProgress>,
    stage_catalog: Res<StageCatalog>,
    localization: Res<Localization>,
    descent_query: Query<(), With<PlayerGoalDescent>>,
) {
    if !editor_state.stage_cleared || !descent_query.is_empty() {
        return;
    }

    progress.unlock_until(StageId(progression.current_stage_id().0 + 1));

    if progression.advance(&stage_catalog) {
        let stage_label = progression
            .current_stage()
            .map(|stage| localized_stage_name(&localization, stage.id, &stage.title))
            .unwrap_or_else(|| format!("STAGE-{}", progression.current_stage_id().0));
        editor_state.last_run_feedback = Some(tr_with_args(
            &localization,
            "stage-ui-feedback-advance",
            &[("stage", stage_label.as_str())],
        ));
        info!("Advancing to next stage");
        editor_state.controls_enabled = false;
        editor_state.pending_player_reset = false;
    } else {
        info!("No more stages to advance to");
        editor_state.controls_enabled = false;
        editor_state.pending_player_reset = false;
        editor_state.last_run_feedback = Some(tr(&localization, "stage-ui-feedback-complete"));
    }

    editor_state.stage_cleared = false;
}

#[derive(SystemParam)]
pub struct StageReloadParams<'w, 's> {
    asset_store: Res<'w, AssetStore>,
    viewport: Res<'w, ScaledViewport>,
    letterbox_offsets: Res<'w, LetterboxOffsets>,
    asset_server: Res<'w, AssetServer>,
    atlas_layouts: ResMut<'w, Assets<TextureAtlasLayout>>,
    tiled_map_assets: Res<'w, TiledMapAssets>,
    window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    progression: ResMut<'w, StageProgressionState>,
    stage_roots: Query<'w, 's, Entity, With<StageRoot>>,
    query: Query<'w, 's, Entity, StageCleanupFilter>,
    tiles: Query<'w, 's, Entity, With<StageTile>>,
    stones: Query<'w, 's, Entity, With<StoneRune>>,
    tutorial_overlays: Query<'w, 's, Entity, With<StageTutorialOverlay>>,
    editor_state: Option<ResMut<'w, ScriptEditorState>>,
    localization: Res<'w, Localization>,
    audio_state: Option<ResMut<'w, StageAudioState>>,
}

pub fn reload_stage_if_needed(mut commands: Commands, mut params: StageReloadParams) {
    if !params.progression.take_pending_reload() {
        return;
    }

    let stage_id = params.progression.current_stage_id();
    let stage_label = params
        .progression
        .current_stage()
        .map(|stage| localized_stage_name(&params.localization, stage.id, &stage.title))
        .unwrap_or_else(|| format!("STAGE-{}", stage_id.0));
    let current_map = params.progression.current_map();

    cleanup_stage_entities(
        &mut commands,
        &params.stage_roots,
        &params.query,
        &params.tiles,
        &params.stones,
        &params.tutorial_overlays,
    );

    if let Some(audio_state) = params.audio_state.as_deref_mut() {
        audio_state.reset(&mut commands);
    } else {
        commands.insert_resource(StageAudioState::default());
    }

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
        info!("Setting up editor state for reloaded stage");
        editor.controls_enabled = false;
        editor.pending_player_reset = false;
        editor.stage_cleared = false;
        editor.set_tutorial_for_stage(stage_id);
        editor.set_command_help_for_stage(stage_id);
        editor.last_run_feedback = Some(tr_with_args(
            &params.localization,
            "stage-ui-feedback-start",
            &[("stage", stage_label.as_str())],
        ));
    }

    let tutorial_dialog = params
        .editor_state
        .as_ref()
        .and_then(|editor| editor.tutorial_dialog.clone())
        .or_else(|| ui::tutorial_dialog_for_stage(stage_id));
    if let Some(dialog) = tutorial_dialog {
        ui::spawn_tutorial_overlay(
            &mut commands,
            params.asset_store.as_ref(),
            params.localization.as_ref(),
            &dialog,
            params.letterbox_offsets.as_ref(),
        );
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

const DISABLED_SCENE_SATURATION: f32 = 0.01;
const SATURATION_EPSILON: f32 = 0.005;

pub fn update_stage_color_grading(
    editor_state: Option<Res<ScriptEditorState>>,
    mut camera_query: Query<&mut ColorGrading, With<MainCamera>>,
) {
    let Ok(mut color_grading) = camera_query.single_mut() else {
        return;
    };

    let controls_enabled = editor_state
        .as_ref()
        .map(|state| state.controls_enabled)
        .unwrap_or(true);

    let target_saturation = if controls_enabled {
        1.0
    } else {
        DISABLED_SCENE_SATURATION
    };

    if (color_grading.global.post_saturation - target_saturation).abs() > SATURATION_EPSILON {
        color_grading.global.post_saturation = target_saturation;
    }

    for section in color_grading.all_sections_mut() {
        if (section.saturation - target_saturation).abs() > SATURATION_EPSILON {
            section.saturation = target_saturation;
        }
    }
}
