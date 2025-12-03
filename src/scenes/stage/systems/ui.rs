use bevy::{input::ButtonInput, prelude::*};
use bevy_ecs::system::SystemParam;
use bevy_egui::{
    EguiContexts,
    egui::{
        self, Align2, FontFamily::Proportional, FontId, FontSelection, Id, Layout, RichText,
        TextFormat, TextStyle,
        text::LayoutJob,
    },
};
use bevy_fluent::prelude::Localization;

use crate::{
    resources::{
        asset_store::AssetStore,
        design_resolution::LetterboxOffsets,
        game_state::GameState,
        script_engine::{Language, ScriptExecutor},
        settings::GameSettings,
        stage_catalog::StageId,
    },
    scenes::{
        assets::FontKey,
        audio::{AudioHandles, play_ui_click},
        stage::systems::{StoneAppendCommandMessage, StoneCommandMessage},
    },
    util::{
        localization::{script_error_message, tr, tr_with_args},
        script_types::ScriptProgram,
    },
};

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

pub fn init_editor_state(commands: &mut Commands, stage_id: StageId) {
    let mut editor_state = ScriptEditorState {
        buffer: String::from(""),
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
    tutorial_overlays: Query<'w, 's, Entity, With<StageTutorialOverlay>>,
}

pub fn ui(params: StageUIParams, mut not_first: Local<bool>) {
    let StageUIParams {
        mut commands,
        mut contexts,
        mut letterbox_offsets,
        mut editor,
        script_executor,
        localization,
        mut stone_writer,
        mut next_state,
        audio,
        settings,
        tutorial_overlays,
    } = params;
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut action_from_keys = None;
    ctx.input(|input| {
        for action in EditorMenuAction::ALL {
            if let Some(key) = action.key()
                && input.key_pressed(key)
            {
                action_from_keys = Some(action);
                break;
            }
        }
    });

    let screen_width = ctx.input(|input| input.content_rect().width());

    let mut style = (*ctx.style()).clone();
    if !*not_first {
        style.text_styles.iter().for_each(|s| {
            info!("Text style: {:?} => {:?}", s.0, s.1);
        });
        *not_first = true;
    }
    let font_offset = editor.font_offset;
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(scaled_panel_font_size(16.0, font_offset), Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(scaled_panel_font_size(10.0, font_offset), Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(scaled_panel_font_size(10.0, font_offset), Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(scaled_panel_font_size(10.0, font_offset), Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(scaled_panel_font_size(8.0, font_offset), Proportional),
        ),
    ]
    .into();
    ctx.set_style(style);

    let (min_width, max_width) = if screen_width.is_finite() && screen_width > 0.0 {
        (screen_width * 0.125, screen_width * 0.5)
    } else {
        (100.0, 300.0)
    };

    let stored_width = screen_width * 0.25;
    let default_width = if stored_width > 0.0 {
        stored_width.clamp(min_width, max_width)
    } else {
        ((min_width + max_width) * 0.5).clamp(min_width, max_width)
    };

    let left = egui::SidePanel::left("stage-left")
        .resizable(true)
        .default_width(default_width)
        .min_width(min_width)
        .max_width(max_width)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(0xe0, 0xe1, 0xe4),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 150)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                let back_label = tr(&localization, "stage-ui-back-to-title");
                if ui.button(back_label.as_str()).clicked() {
                    play_ui_click(&mut commands, &audio, &settings);

                    editor.controls_enabled = false;
                    editor.pending_player_reset = false;
                    editor.stage_cleared = false;
                    editor.stage_clear_popup_open = false;
                    next_state.set(GameState::SelectStage);
                }

                ui.separator();

                let mut pending_action = action_from_keys.map(|action| (action, false));

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    for action in EditorMenuAction::ALL {
                        let button_label =
                            tr(&localization, action.label_key(editor.controls_enabled));
                        let label = if let Some(key_text) = action.key_text() {
                            format!("{button_label} ({key_text})")
                        } else {
                            button_label
                        };
                        if ui.button(label).clicked() {
                            play_ui_click(&mut commands, &audio, &settings);
                            pending_action = Some((action, true));
                        }
                    }
                });

                if let Some((action, triggered_via_ui)) = pending_action {
                    if !triggered_via_ui {
                        play_ui_click(&mut commands, &audio, &settings);
                    }
                    let was_running = editor.controls_enabled;
                    let mut action_context_flag = false;
                    match action {
                        EditorMenuAction::RunScript => {
                            if was_running {
                                info!("Stopping script execution");
                                editor.controls_enabled = false;
                                editor.pending_player_reset = true;
                                editor.last_run_feedback =
                                    Some(tr(&localization, "stage-ui-feedback-stopped"));
                                editor.stage_cleared = false;
                                editor.stage_clear_popup_open = false;
                            } else {
                                hide_tutorial_overlays(&mut commands, &tutorial_overlays);
                                let language = settings.script_language;
                                match script_executor.compile_step(language, &editor.buffer) {
                                    Ok(program) => {
                                        // Clear any existing queue on the Stone
                                        stone_writer
                                            .write(StoneCommandMessage { commands: vec![] });

                                        editor.active_program = Some(program);
                                        editor.last_run_feedback = Some(tr(
                                            &localization,
                                            "stage-ui-feedback-step-started",
                                        ));
                                        editor.controls_enabled = true;
                                        editor.pending_player_reset = true;
                                        editor.stage_cleared = false;
                                        editor.stage_clear_popup_open = false;
                                    }
                                    Err(err) => {
                                        editor.active_program = None;
                                        editor.last_run_feedback =
                                            Some(script_error_message(&localization, &err));
                                        info!("Script compilation error: {}", err);
                                        editor.controls_enabled = false;
                                        editor.pending_player_reset = false;
                                        editor.stage_cleared = false;
                                        editor.stage_clear_popup_open = false;
                                        warn!("Failed to compile script: {}", err);
                                    }
                                }
                            }
                            action_context_flag = was_running;
                        }
                        EditorMenuAction::DecreaseFont => {
                            editor.font_offset = (editor.font_offset - FONT_OFFSET_STEP)
                                .clamp(FONT_OFFSET_MIN, FONT_OFFSET_MAX);
                        }
                        EditorMenuAction::IncreaseFont => {
                            editor.font_offset = (editor.font_offset + FONT_OFFSET_STEP)
                                .clamp(FONT_OFFSET_MIN, FONT_OFFSET_MAX);
                        }
                        EditorMenuAction::ToggleCommandHelp => {
                            if let Some(help) = editor.command_help.as_mut() {
                                help.is_open = !help.is_open;
                                action_context_flag = help.is_open;
                            }
                        }
                    }
                    editor.apply_action(action, action_context_flag);
                }

                if let Some(action) = editor.last_action {
                    info!("Editor action: {:?}", action);
                    // let status = tr(&localization, action.status_key(editor.last_action_context));
                    // ui.label(status);
                    editor.last_action = None;
                }

                if let Some(feedback) = &editor.last_run_feedback {
                    ui.label(feedback);
                }

                ui.separator();

                let mut available_size = ui.available_size();
                if !available_size.x.is_finite() {
                    available_size.x = ui.max_rect().width();
                }
                if !available_size.y.is_finite() {
                    available_size.y = ui.max_rect().height();
                }

                // Animate the help drawer so it grows from the bottom when opened.
                let help_is_open = editor
                    .command_help
                    .as_ref()
                    .is_some_and(|help| help.is_open);
                let help_anim = ui
                    .ctx()
                    .animate_bool(Id::new("command-help-open"), help_is_open);
                let help_target_height = 220.0;
                let help_height = help_anim * help_target_height;

                let text_height = (available_size.y - help_height).max(160.0);
                let font_size = (BASE_EDITOR_FONT_SIZE + editor.font_offset)
                    .clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE);
                let editing_locked = editor.controls_enabled;

                let text_edit_response = ui.add_sized(
                    egui::Vec2::new(available_size.x, text_height),
                    egui::TextEdit::multiline(&mut editor.buffer)
                        .font(FontSelection::FontId(FontId::new(
                            font_size,
                            egui::FontFamily::Name("pixel_mplus".into()),
                        )))
                        .code_editor()
                        .interactive(!editing_locked)
                        .desired_width(f32::INFINITY),
                );

                if text_edit_response.changed() {
                    info!("Script editor buffer changed");
                    editor.controls_enabled = false;
                    editor.stage_cleared = false;
                    editor.stage_clear_popup_open = false;
                }

                if help_is_open || help_height > 1.0 {
                    let mut remaining = ui.available_size();
                    if !remaining.x.is_finite() {
                        remaining.x = ui.max_rect().width();
                    }
                    // Reserve animated height so the help rises from the bottom.
                    remaining.y = help_height;

                    if let Some(help) = editor.command_help.as_ref().filter(|h| h.is_open) {
                        let font_id = FontId::new(
                            scaled_panel_font_size(10.0, editor.font_offset),
                            Proportional,
                        );

                        ui.allocate_ui_with_layout(
                            egui::Vec2::new(remaining.x, remaining.y),
                            Layout::bottom_up(egui::Align::LEFT),
                            |ui| {
                                ui.add_space(8.0);
                                let content_height = (remaining.y - 8.0).max(0.0);

                                egui::Frame::group(ui.style())
                                    .fill(egui::Color32::from_black_alpha(200))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(80)))
                                    .inner_margin(egui::Margin::symmetric(12, 10))
                                    .show(ui, |ui| {
                                        ui.set_min_height(content_height);
                                        ui.set_max_height(content_height);

                                        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(
                                            ui,
                                            |ui| {
                                                let command_help_args =
                                                    command_help_args(settings.script_language);
                                                ui.vertical(|ui| {
                                                    let title = tr(&localization, help.title_key);
                                                    ui.label(
                                                        RichText::new(title)
                                                            .strong()
                                                            .font(font_id.clone()),
                                                    );
                                                    ui.add_space(6.0);
                                                    let entry = tr_with_args(
                                                        &localization,
                                                        &help.entry,
                                                        command_help_args,
                                                    );
                                                    let entry_job =
                                                        highlight_backtick_segments(&entry, &font_id, ui);
                                                    ui.label(entry_job);
                                                    ui.add_space(4.0);
                                                });
                                            },
                                        );
                                    });
                            },
                        );
                    } else {
                        // Keep layout space during the closing animation.
                        ui.allocate_space(egui::Vec2::new(remaining.x, remaining.y));
                    }
                }
            });
        })
        .response
        .rect
        .width()
        .clamp(min_width, max_width);

    if editor.stage_clear_popup_open {
        let mut popup_open = editor.stage_clear_popup_open;
        let mut request_close = false;
        let window_title = tr(&localization, "stage-ui-clear-window-title");
        egui::Window::new(window_title)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .open(&mut popup_open)
            .show(ctx, |ui| {
                let heading = tr(&localization, "stage-ui-clear-heading");
                ui.heading(heading);
                ui.add_space(8.0);
                let body = tr(&localization, "stage-ui-clear-body");
                ui.label(body);
                ui.add_space(12.0);
                let ok = tr(&localization, "stage-ui-clear-ok");
                if ui.button(ok.as_str()).clicked() {
                    play_ui_click(&mut commands, &audio, &settings);
                    request_close = true;
                }
            });

        editor.stage_clear_popup_open = popup_open && !request_close;
    }

    // // Draw in-stage (non-popup) clear banner while stage_cleared is true.
    // if editor.stage_cleared {
    //     let banner = tr(&localization, "stage-ui-feedback-goal");
    //     let font_id = FontId::new(48.0, Proportional);
    //     let painter = ctx.layer_painter(egui::LayerId::new(
    //         egui::Order::Foreground,
    //         egui::Id::new("stage_clear_banner"),
    //     ));
    //     // Use content_rect per updated egui API.
    //     let center = ctx.content_rect().center();
    //     painter.text(
    //         center,
    //         Align2::CENTER_CENTER,
    //         banner,
    //         font_id,
    //         egui::Color32::from_rgb(255, 255, 200),
    //     );
    // }

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
        info!(
            "Updating letterbox offsets: left={} (was {})",
            left, letterbox_offsets.left
        );
        letterbox_offsets.left = left;
    }
}

/// Each frame, pull at most one next command from the active program and append it to the Stone.
pub fn tick_script_program(
    mut editor: ResMut<ScriptEditorState>,
    mut append_writer: MessageWriter<StoneAppendCommandMessage>,
) {
    if !editor.controls_enabled {
        editor.active_program = None;
        return;
    }

    let Some(program) = editor.active_program.as_mut() else {
        return;
    };

    if let Some(command) = program.next() {
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

pub fn tutorial_dialog_for_stage(stage_id: StageId) -> Option<TutorialDialog> {
    if stage_id.0 <= 3 {
        let id = stage_id.0;
        Some(TutorialDialog::new(
            format!("stage{}-name", id),
            format!("stage{}-text", id),
        ))
    } else {
        None
    }
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
        ],
        Language::Keystone => &[
            ("move-up", "move up"),
            ("move-down", "move down"),
            ("move-right", "move right"),
            ("move-left", "move left"),
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
        commands.entity(entity).despawn();
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
    let body = tr(localization, &dialog.body_key);
    let chunks = chunk_tutorial_text(&body);
    if chunks.is_empty() {
        return;
    }
    let hint = tr(localization, "stage-ui-tutorial-next-hint");
    let mut body_value = chunks[0].clone();
    if chunks.len() > 1 {
        body_value.push_str("\n\n");
    }

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
        && let Ok((_, mut overlay)) = tutorial_overlays.single_mut()
    {
        overlay.padding.left = Val::Px(letterbox_offsets.left);
    }

    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    if let Ok((entity, mut overlay)) = tutorial_overlay_panels.single_mut() {
        if overlay.advance() {
            if let Ok(mut text) = texts.get_mut(overlay.body_entity) {
                update_overlay_text(&overlay, &mut text);
            }
            if overlay.next_is_last() {
                tutorial_hints
                    .iter_mut()
                    .for_each(|(entity, mut _hint_text)| {
                        commands.entity(entity).despawn();
                    });
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
}
