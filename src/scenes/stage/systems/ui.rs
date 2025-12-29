use avian2d::prelude::*; // Import all avian2d prelude items including SpatialQuery, LayerMask, CollidingEntities
use bevy::{input::ButtonInput, prelude::*};
use bevy_ecs::system::SystemParam;
use bevy_egui::{
    EguiContexts,
    egui::{
        self, Align2, FontFamily::Monospace, FontFamily::Proportional, FontId, FontSelection, Id,
        Layout, RichText, TextFormat, TextStyle, text::LayoutJob,
    },
};
use bevy_fluent::prelude::Localization;

use super::stone::StoneCommandState;
use crate::scenes::stage::systems::StageProgressionState;
use crate::{
    resources::{
        asset_store::AssetStore,
        design_resolution::LetterboxOffsets,
        file_storage::FileStorageResource,
        game_state::GameState,
        script_engine::{Language, ScriptExecutor},
        settings::GameSettings,
        stage_catalog::StageId,
        stage_scripts::StageScripts,
        stone_type::{StoneCapabilities, StoneType},
    },
    scenes::{
        assets::FontKey,
        audio::{AudioHandles, play_ui_click},
        stage::{components::*, systems::*},
    },
    util::{
        localization::{script_error_message, tr, tr_or, tr_with_args},
        script_types::{
            PLAYER_TOUCHED_STATE_KEY, RAND_STATE_KEY, ScriptProgram, ScriptState, ScriptStateValue,
        },
    },
};
use rand::Rng;

#[derive(Clone, Debug)]
pub struct TutorialDialog {
    pub title_key: String,
    pub body_key: String,
}

impl TutorialDialog {
    fn new(title_key: String, body_key: String) -> Self {
        Self {
            title_key,
            body_key,
        }
    }
}

#[derive(Component)]
pub struct StageTutorialOverlay;

#[derive(Component)]
pub struct StageTutorialHint;

#[derive(Component)]
pub struct TutorialOverlayPanel {
    chunks: Vec<String>,
    current_chunk: usize,
    body_entity: Entity,
}

impl TutorialOverlayPanel {
    fn has_next(&self) -> bool {
        self.current_chunk + 1 < self.chunks.len()
    }

    fn next_is_last(&self) -> bool {
        self.current_chunk + 1 == self.chunks.len()
    }

    fn advance(&mut self) -> bool {
        if self.has_next() {
            self.current_chunk += 1;
            true
        } else {
            false
        }
    }

    fn current_text(&self) -> &str {
        self.chunks
            .get(self.current_chunk)
            .map(String::as_str)
            .unwrap_or("")
    }
}

#[derive(Clone, Debug)]
pub struct CommandHelpDialog {
    title_key: &'static str,
    entry: String,
    is_open: bool,
}

impl CommandHelpDialog {
    fn new(title_key: &'static str, entry: String) -> Self {
        Self {
            title_key,
            entry,
            is_open: false,
        }
    }
}

const BASE_EDITOR_FONT_SIZE: f32 = 10.0;
const MIN_EDITOR_FONT_SIZE: f32 = 8.0;
const MAX_EDITOR_FONT_SIZE: f32 = 20.0;
const FONT_OFFSET_STEP: f32 = 1.0;
const FONT_OFFSET_MIN: f32 = -6.0;
const FONT_OFFSET_MAX: f32 = 0.0;

fn scaled_panel_font_size(base: f32, offset: f32) -> f32 {
    ((base + offset).max(4.0)) * 2.0
}

#[derive(Resource)]
pub struct ScriptEditorState {
    pub buffer: String,
    pub last_action: Option<EditorMenuAction>,
    pub last_action_context: bool,
    pub last_run_feedback: Option<String>,
    pub active_program: Option<Box<dyn ScriptProgram>>,
    pub controls_enabled: bool,
    pub pending_player_reset: bool,
    pub stage_cleared: bool,
    pub stage_clear_popup_open: bool,
    pub tutorial_dialog: Option<TutorialDialog>,
    pub command_help: Option<CommandHelpDialog>,
    pub font_offset: f32,
}

impl Default for ScriptEditorState {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            last_action: None,
            last_action_context: false,
            last_run_feedback: None,
            active_program: None,
            controls_enabled: false,
            pending_player_reset: false,
            stage_cleared: false,
            stage_clear_popup_open: false,
            tutorial_dialog: None,
            command_help: None,
            font_offset: 0.0,
        }
    }
}

impl ScriptEditorState {
    fn apply_action(&mut self, action: EditorMenuAction, context: bool) {
        self.last_action = Some(action);
        self.last_action_context = context;
    }

    pub fn set_tutorial_for_stage(&mut self, stage_id: StageId) {
        self.tutorial_dialog = tutorial_dialog_for_stage(stage_id);
    }

    pub fn set_command_help_for_stage(&mut self, stage_id: StageId) {
        self.command_help = command_help_for_stage(stage_id);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMenuAction {
    RunScript,
    DecreaseFont,
    IncreaseFont,
    ToggleCommandHelp,
}

impl EditorMenuAction {
    const ALL: [Self; 4] = [
        Self::DecreaseFont,
        Self::IncreaseFont,
        Self::RunScript,
        Self::ToggleCommandHelp,
    ];

    fn label_key(self, is_running: bool) -> &'static str {
        match self {
            Self::DecreaseFont => "stage-ui-menu-font-decrease",
            Self::IncreaseFont => "stage-ui-menu-font-increase",
            Self::RunScript if is_running => "stage-ui-menu-stop",
            Self::RunScript => "stage-ui-menu-run",
            Self::ToggleCommandHelp => "stage-ui-command-help-button",
        }
    }

    fn key_text(self) -> Option<&'static str> {
        match self {
            Self::DecreaseFont => Some("F1"),
            Self::IncreaseFont => Some("F2"),
            Self::RunScript => Some("F3"),
            Self::ToggleCommandHelp => Some("F4"),
        }
    }

    fn key(self) -> Option<egui::Key> {
        match self {
            Self::DecreaseFont => Some(egui::Key::F1),
            Self::IncreaseFont => Some(egui::Key::F2),
            Self::RunScript => Some(egui::Key::F3),
            Self::ToggleCommandHelp => Some(egui::Key::F4),
        }
    }
}

pub fn init_editor_state(commands: &mut Commands, stage_id: StageId, saved_code: Option<String>) {
    let mut editor_state = ScriptEditorState {
        buffer: saved_code.unwrap_or_default(),
        ..default()
    };
    editor_state.set_tutorial_for_stage(stage_id);
    editor_state.set_command_help_for_stage(stage_id);
    commands.insert_resource(editor_state);
}

#[derive(SystemParam)]
pub struct StageUIParams<'w, 's> {
    commands: Commands<'w, 's>,
    contexts: EguiContexts<'w, 's>,
    letterbox_offsets: ResMut<'w, LetterboxOffsets>,
    editor: ResMut<'w, ScriptEditorState>,
    script_executor: Res<'w, ScriptExecutor>,
    localization: Res<'w, Localization>,
    stone_writer: MessageWriter<'w, StoneCommandMessage>,
    next_state: ResMut<'w, NextState<GameState>>,
    audio: Res<'w, AudioHandles>,
    settings: Res<'w, GameSettings>,
    stage_scripts: ResMut<'w, StageScripts>,
    progression: Res<'w, StageProgressionState>,
    tutorial_overlays: Query<'w, 's, Entity, With<StageTutorialOverlay>>,
    stone_capabilities: Res<'w, StoneCapabilities>,
    stone_query:
        Query<'w, 's, (Entity, &'static GlobalTransform, &'static StoneType), With<StoneRune>>,
    file_storage: Res<'w, FileStorageResource>,
}

pub fn ui(_params: StageUIParams, mut _not_first: Local<bool>) {
    info!("UI system heart beat");
    info!("UI system finished iteration");
}

/// Each frame, pull at most one next command from the active program and append it to the Stone.
pub fn tick_script_program(
    mut editor: ResMut<ScriptEditorState>,
    mut append_writer: MessageWriter<StoneAppendCommandMessage>,
    players: Query<(Entity, &CollidingEntities), With<Player>>,
    stone_query: Query<(Entity, &GlobalTransform, &StoneType), With<StoneRune>>,
    stone_states: Query<&StoneCommandState, With<StoneRune>>,
    tiles: Query<(), With<StageTile>>,
    spatial: SpatialQuery,
) {
    info!("tick_script_program heart beat");
    if !editor.controls_enabled {
        editor.active_program = None;
        return;
    }

    let Some(program) = editor.active_program.as_mut() else {
        return;
    };

    let Some((stone_entity, stone_transform, _)) = stone_query.iter().next() else {
        return;
    };

    if let Ok(stone_state) = stone_states.get(stone_entity)
        && stone_state.is_busy()
    {
        // Wait until the stone finishes its current action to avoid
        // queueing stale commands based on old touch state.
        return;
    }

    // Optimization: check player touch first.
    // User requested "only move when touching".
    let player_touched = is_player_touching_stone(&players, stone_entity);

    // Reverted optimization: The strict check prevented non-touch scripts from running.
    // Instead we will handle "double move" via a cooldown in stone.rs.

    let mut state = ScriptState::default();
    state.insert(
        PLAYER_TOUCHED_STATE_KEY.to_string(),
        ScriptStateValue::Bool(player_touched),
    );
    state.insert(
        RAND_STATE_KEY.to_string(),
        ScriptStateValue::Float(rand::rng().random_range(0.0..1.0)),
    );

    // Calculate surrounding state using shape cast
    // We check if the stone can move one full step without hitting a wall
    let directions = [
        ("up", Vec2::Y),
        ("down", Vec2::NEG_Y),
        ("left", Vec2::NEG_X),
        ("right", Vec2::X),
    ];
    let stone_scale = stone_transform.scale().x;

    // Get step_size from stone state if available, or use default
    let step_size = stone_states
        .get(stone_entity)
        .map(|s| s.step_size)
        .unwrap_or(super::stone::STONE_STEP_DISTANCE);

    // Check distance = stone collider radius + a small margin
    // This detects if the stone's edge is already touching or very close to a wall
    let collider_radius = super::stone::STONE_COLLIDER_RADIUS * stone_scale;
    let check_dist = step_size * stone_scale; // Check for one full step distance
    let origin = stone_transform.translation().truncate();
    // Collect player entities to exclude from collision detection
    let player_entities: Vec<Entity> = players.iter().map(|(e, _)| e).collect();
    let mut excluded_entities = vec![stone_entity];
    excluded_entities.extend(player_entities);
    let filter =
        SpatialQueryFilter::from_mask(LayerMask::ALL).with_excluded_entities(excluded_entities);
    // Shape cast with a circle matching the stone's collider size
    let cast_shape = Collider::circle(collider_radius);
    let cast_config = ShapeCastConfig::from_max_distance(check_dist);

    for (name, dir) in directions {
        let ray_dir = Dir2::new(dir).expect("Invalid direction");
        // Use shape cast to check if stone can move one step without collision
        let hit = spatial.cast_shape(&cast_shape, origin, 0.0, ray_dir, &cast_config, &filter);
        let is_blocked = hit.is_some_and(|h| tiles.get(h.entity).is_ok());
        state.insert(
            format!("is-empty-{}", name),
            ScriptStateValue::Bool(!is_blocked),
        );
        info!(
            "is-empty-{}: {} (origin={:?}, radius={}, step={}, hit={:?})",
            name,
            !is_blocked,
            origin,
            collider_radius,
            check_dist,
            hit.map(|h| (h.distance, h.entity))
        );
    }

    if let Some(command) = program.next(&state) {
        append_writer.write(StoneAppendCommandMessage {
            command: command.clone(),
        });
    } else {
        // // Program exhausted: stop execution.
        // info!("Script program completed");
        // editor.controls_enabled = false;
        // editor.active_program = None;
    }
}

fn is_player_touching_stone(
    players: &Query<(Entity, &CollidingEntities), With<Player>>,
    stone_entity: Entity,
) -> bool {
    let Some((_, player_collisions)) = players.iter().next() else {
        return false;
    };

    player_collisions
        .iter()
        .any(|&entity| entity == stone_entity)
}

pub fn tutorial_dialog_for_stage(stage_id: StageId) -> Option<TutorialDialog> {
    let id = stage_id.0;
    Some(TutorialDialog::new(
        format!("stage{}-name", id),
        format!("stage{}-text", id),
    ))
}

fn command_help_for_stage(stage_id: StageId) -> Option<CommandHelpDialog> {
    Some(CommandHelpDialog::new(
        "stage-ui-command-help-title",
        format!("stage{}-description", stage_id.0),
    ))
}

fn command_help_args(language: Language) -> &'static [(&'static str, &'static str)] {
    match language {
        Language::Rhai => &[
            ("move-up", "move(\"up\");"),
            ("move-down", "move(\"down\");"),
            ("move-right", "move(\"right\");"),
            ("move-left", "move(\"left\");"),
            ("sleep-1", "sleep(1);"),
            ("sleep-2x5", "sleep(2<<dot>>5);"), // https://github.com/kgv/fluent_content/issues/3
            ("loop-example", "loop {\n    move(\"up\");\n}"),
            (
                "loop-example2",
                "loop {\n    move(\"up\");\n    sleep(1);\n}",
            ),
            (
                "touched-example",
                "loop {\n    if is_touched() {\n        <<dot>><<dot>><<dot>>\n    }\n}",
            ),
        ],
        Language::Keystone => &[
            ("move-up", "move up"),
            ("move-down", "move down"),
            ("move-right", "move right"),
            ("move-left", "move left"),
            ("sleep-1", "sleep 1"),
            ("sleep-2x5", "sleep 2<<dot>>5"),
        ],
    }
}

fn highlight_backtick_segments(text: &str, font_id: &FontId, ui: &egui::Ui) -> LayoutJob {
    let mut job = LayoutJob::default();
    let normal_format = TextFormat {
        font_id: font_id.clone(),
        color: ui.visuals().strong_text_color(),
        ..Default::default()
    };
    let highlight_format = TextFormat {
        font_id: FontId::new(font_id.size, egui::FontFamily::Monospace),
        color: egui::Color32::from_rgb(240, 220, 140),
        ..Default::default()
    };

    let mut in_highlight = false;
    for segment in text.split('`') {
        let format = if in_highlight {
            highlight_format.clone()
        } else {
            normal_format.clone()
        };
        job.append(segment, 0.0, format);
        in_highlight = !in_highlight;
    }

    job
}

fn chunk_tutorial_text(input: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                chunks.push(current.join("\n"));
                current.clear();
            }
        } else if line == "____" {
            current.push("".to_string());
        } else {
            current.push(line.trim().to_string());
        }
    }

    if !current.is_empty() {
        chunks.push(current.join("\n"));
    }

    chunks
}

fn update_overlay_text(panel: &TutorialOverlayPanel, text: &mut Text) {
    text.0.clear();
    text.0.push_str(panel.current_text());
    if panel.has_next() {
        text.0.push_str("\n\n");
    }
}

fn hide_tutorial_overlays(
    commands: &mut Commands,
    overlays: &Query<Entity, With<StageTutorialOverlay>>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).try_despawn();
    }
}

pub fn spawn_tutorial_overlay(
    commands: &mut Commands,
    asset_store: &AssetStore,
    localization: &Localization,
    dialog: &TutorialDialog,
    letterbox_offsets: &LetterboxOffsets,
) {
    let Some(font) = asset_store.font(FontKey::Default) else {
        warn!("Tutorial overlay: default font is missing");
        return;
    };

    let title = tr(localization, &dialog.title_key);
    let body = tr_or(localization, &dialog.body_key, "");
    let chunks = chunk_tutorial_text(&body);
    let mut body_value = if chunks.is_empty() {
        String::new()
    } else {
        chunks[0].clone()
    };
    if chunks.len() > 1 {
        body_value.push_str("\n\n");
    }
    let hint = tr(localization, "stage-ui-tutorial-next-hint");

    let mut body_entity = None;
    let overlay_entity = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                padding: UiRect {
                    left: Val::Px(letterbox_offsets.left),
                    right: Val::Px(letterbox_offsets.right),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(5),
            StageTutorialOverlay,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(460.0),
                        padding: UiRect {
                            left: Val::Px(18.0),
                            right: Val::Px(18.0),
                            top: Val::Px(16.0),
                            bottom: Val::Px(18.0),
                        },
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexEnd,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.75)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            ..default()
                        },
                        Text::new(title),
                        TextFont {
                            font: font.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextLayout::new(Justify::Left, LineBreak::WordBoundary),
                        TextColor(Color::srgb(0.95, 0.9, 0.65)),
                    ));

                    let entity = panel
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                ..default()
                            },
                            Text::new(body_value),
                            TextFont {
                                font: font.clone(),
                                font_size: 20.0,
                                ..default()
                            },
                            TextLayout::new(Justify::Left, LineBreak::WordBoundary),
                            TextColor(Color::srgb(0.95, 0.95, 0.95)),
                        ))
                        .id();
                    body_entity = Some(entity);

                    panel.spawn((
                        StageTutorialHint,
                        Node {
                            width: Val::Percent(100.0),
                            ..default()
                        },
                        Text::new(hint.clone()),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextLayout::new(Justify::Right, LineBreak::WordBoundary),
                        TextColor(Color::srgb(0.95, 0.95, 0.95)),
                    ));
                });
        })
        .id();

    if let Some(body_entity) = body_entity {
        commands
            .entity(overlay_entity)
            .insert(TutorialOverlayPanel {
                chunks,
                current_chunk: 0,
                body_entity,
            });
    }
}

pub fn handle_tutorial_overlay_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut tutorial_overlays: Query<(&StageTutorialOverlay, &mut Node)>,
    mut tutorial_overlay_panels: Query<
        (Entity, &mut TutorialOverlayPanel),
        With<StageTutorialOverlay>,
    >,
    mut tutorial_hints: Query<(Entity, &mut StageTutorialHint)>,
    mut texts: Query<&mut Text>,
    letterbox_offsets: ResMut<LetterboxOffsets>,
) {
    if letterbox_offsets.is_changed()
        && let Some((_, mut overlay)) = tutorial_overlays.iter_mut().next()
    {
        overlay.padding.left = Val::Px(letterbox_offsets.left);
    }

    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    if let Some((entity, mut overlay)) = tutorial_overlay_panels.iter_mut().next() {
        if overlay.advance() {
            if let Ok(mut text) = texts.get_mut(overlay.body_entity) {
                update_overlay_text(&overlay, &mut text);
            }
            if overlay.next_is_last() {
                tutorial_hints
                    .iter_mut()
                    .for_each(|(entity, mut _hint_text)| {
                        commands.entity(entity).try_despawn();
                    });
            }
        } else {
            commands.entity(entity).try_despawn();
        }
    }
}
