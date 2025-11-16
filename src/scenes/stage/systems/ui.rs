use bevy::{input::ButtonInput, prelude::*};
use bevy_ecs::system::SystemParam;
use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Event, FontId, FontSelection, RichText},
};
use bevy_fluent::prelude::Localization;

use crate::{
    resources::{
        asset_store::AssetStore,
        design_resolution::LetterboxOffsets,
        game_state::GameState,
        script_engine::{Language, ScriptExecutor},
        stage_catalog::StageId,
    },
    scenes::{
        assets::FontKey,
        audio::{AudioHandles, play_ui_click},
        stage::systems::{StoneAppendCommandMessage, StoneCommandMessage},
    },
    util::{
        localization::{script_error_message, tr, tr_with_args},
        script_types::{ScriptCommand, ScriptProgram},
    },
};

#[derive(Clone, Debug)]
pub struct TutorialDialog {
    pub title_key: &'static str,
    pub body_key: &'static str,
}

impl TutorialDialog {
    fn new(title_key: &'static str, body_key: &'static str) -> Self {
        Self {
            title_key,
            body_key,
        }
    }
}

#[derive(Component)]
pub struct StageTutorialOverlay;

#[derive(Component)]
pub struct TutorialOverlayPanel {
    chunks: Vec<String>,
    current_chunk: usize,
    hint: String,
    body_entity: Entity,
}

impl TutorialOverlayPanel {
    fn has_next(&self) -> bool {
        self.current_chunk + 1 < self.chunks.len()
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

#[derive(Clone, Copy, Debug)]
pub struct CommandHelpEntry {
    title_key: &'static str,
    body_key: &'static str,
}

#[derive(Clone, Debug)]
pub struct CommandHelpDialog {
    title_key: &'static str,
    intro_key: &'static str,
    entries: &'static [CommandHelpEntry],
    is_open: bool,
}

impl CommandHelpDialog {
    fn new(
        title_key: &'static str,
        intro_key: &'static str,
        entries: &'static [CommandHelpEntry],
    ) -> Self {
        Self {
            title_key,
            intro_key,
            entries,
            is_open: false,
        }
    }
}

const DEFAULT_COMMAND_HELP_ENTRIES: &[CommandHelpEntry] = &[
    CommandHelpEntry {
        title_key: "stage-ui-command-help-move-title",
        body_key: "stage-ui-command-help-move-body",
    },
    CommandHelpEntry {
        title_key: "stage-ui-command-help-sleep-title",
        body_key: "stage-ui-command-help-sleep-body",
    },
];

#[derive(Resource, Default)]
pub struct ScriptEditorState {
    pub buffer: String,
    pub last_action: Option<EditorMenuAction>,
    pub last_action_was_running: bool,
    pub last_run_feedback: Option<String>,
    pub last_commands: Vec<ScriptCommand>,
    pub active_program: Option<Box<dyn ScriptProgram>>,
    pub controls_enabled: bool,
    pub pending_player_reset: bool,
    pub stage_cleared: bool,
    pub stage_clear_popup_open: bool,
    pub tutorial_dialog: Option<TutorialDialog>,
    pub command_help: Option<CommandHelpDialog>,
}

impl ScriptEditorState {
    fn apply_action(&mut self, action: EditorMenuAction, was_running: bool) {
        self.last_action = Some(action);
        self.last_action_was_running = was_running;
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
    LoadExample,
    SaveBuffer,
    RunScript,
}

impl EditorMenuAction {
    const ALL: [Self; 3] = [Self::LoadExample, Self::SaveBuffer, Self::RunScript];

    fn label_key(self, is_running: bool) -> &'static str {
        match self {
            Self::LoadExample => "stage-ui-menu-load",
            Self::SaveBuffer => "stage-ui-menu-save",
            Self::RunScript if is_running => "stage-ui-menu-stop",
            Self::RunScript => "stage-ui-menu-run",
        }
    }

    fn key_text(self) -> &'static str {
        match self {
            Self::LoadExample => "F1",
            Self::SaveBuffer => "F2",
            Self::RunScript => "F3",
        }
    }

    fn key(self) -> egui::Key {
        match self {
            Self::LoadExample => egui::Key::F1,
            Self::SaveBuffer => egui::Key::F2,
            Self::RunScript => egui::Key::F3,
        }
    }

    fn status_key(self, was_running: bool) -> &'static str {
        match self {
            Self::LoadExample => "stage-ui-status-load",
            Self::SaveBuffer => "stage-ui-status-save",
            Self::RunScript if was_running => "stage-ui-status-stop",
            Self::RunScript => "stage-ui-status-run",
        }
    }
}

fn describe_command(command: &ScriptCommand) -> String {
    match command {
        ScriptCommand::Move(direction) => format!("move({direction})"),
        ScriptCommand::Sleep(seconds) => format!("sleep({:.2}s)", seconds),
    }
}

fn summarize_commands(commands: &[ScriptCommand]) -> String {
    commands
        .iter()
        .map(describe_command)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(SystemParam)]
pub struct StageUiParams<'w, 's> {
    commands: Commands<'w, 's>,
    contexts: EguiContexts<'w, 's>,
    letterbox_offsets: ResMut<'w, LetterboxOffsets>,
    editor: ResMut<'w, ScriptEditorState>,
    script_executor: Res<'w, ScriptExecutor>,
    localization: Res<'w, Localization>,
    stone_writer: MessageWriter<'w, StoneCommandMessage>,
    next_state: ResMut<'w, NextState<GameState>>,
    audio: Res<'w, AudioHandles>,
}

pub fn init_editor_state(commands: &mut Commands, stage_id: StageId) {
    let mut editor_state = ScriptEditorState {
        buffer: String::from(
            "move(\"left\");\n\
             sleep(1.0);\n\
             move(\"right\");\n\
             sleep(1.0);\n\
             for i in 1..=2 {\n  move(\"up\");\n  sleep(0.5);\n\
             }\n",
        ),
        ..default()
    };
    editor_state.set_tutorial_for_stage(stage_id);
    editor_state.set_command_help_for_stage(stage_id);
    commands.insert_resource(editor_state);
}

pub fn ui(params: StageUiParams) {
    let StageUiParams {
        mut commands,
        mut contexts,
        mut letterbox_offsets,
        mut editor,
        script_executor,
        localization,
        mut stone_writer,
        mut next_state,
        audio,
    } = params;
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let popup_open_now = editor.stage_clear_popup_open;
    let mut close_popup_via_input = false;
    let mut action_from_keys = None;
    ctx.input(|input| {
        if popup_open_now {
            close_popup_via_input = input.events.iter().any(|event| {
                matches!(
                    event,
                    Event::Key {
                        key,
                        pressed: true,
                        ..
                    } if !matches!(
                        key,
                        egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                    )
                )
            });
        } else {
            for action in EditorMenuAction::ALL {
                if input.key_pressed(action.key()) {
                    action_from_keys = Some(action);
                    break;
                }
            }
        }
    });

    if close_popup_via_input {
        editor.stage_clear_popup_open = false;
    }

    let screen_width = ctx.input(|input| input.content_rect().width());

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
                    play_ui_click(&mut commands, &audio);
                    info!("Returning to stage select");
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
                        let label = format!("{} ({})", button_label, action.key_text());
                        if ui.button(label).clicked() {
                            play_ui_click(&mut commands, &audio);
                            pending_action = Some((action, true));
                        }
                    }
                });

                if let Some((action, triggered_via_ui)) = pending_action {
                    if !triggered_via_ui {
                        play_ui_click(&mut commands, &audio);
                    }
                    let was_running = editor.controls_enabled;
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
                                match script_executor.compile_step(Language::Rhai, &editor.buffer) {
                                    Ok(program) => {
                                        // Clear any existing queue on the Stone
                                        stone_writer
                                            .write(StoneCommandMessage { commands: vec![] });

                                        editor.active_program = Some(program);
                                        editor.last_commands.clear();
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
                                        editor.last_commands.clear();
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
                        }
                        EditorMenuAction::LoadExample => {
                            info!("Loading example script into editor");
                            editor.controls_enabled = false;
                            editor.pending_player_reset = false;
                            editor.stage_cleared = false;
                            editor.stage_clear_popup_open = false;
                        }
                        EditorMenuAction::SaveBuffer => {}
                    }
                    editor.apply_action(action, was_running);
                }

                if let Some(action) = editor.last_action {
                    info!("Editor action: {:?}", action);
                    let status = tr(
                        &localization,
                        action.status_key(editor.last_action_was_running),
                    );
                    ui.label(status);
                    editor.last_action = None;
                }

                if let Some(feedback) = &editor.last_run_feedback {
                    ui.label(feedback);
                }

                if !editor.last_commands.is_empty() {
                    let summary = summarize_commands(&editor.last_commands);
                    let label = tr_with_args(
                        &localization,
                        "stage-ui-commands-list",
                        &[("summary", summary.as_str())],
                    );
                    ui.label(label);
                }

                ui.separator();

                let mut available_size = ui.available_size();
                if !available_size.x.is_finite() {
                    available_size.x = ui.max_rect().width();
                }
                if !available_size.y.is_finite() {
                    available_size.y = ui.max_rect().height();
                }

                let reserved_height = 200.0;
                let text_height = (available_size.y - reserved_height).max(160.0);
                let text_edit_response = ui.add_sized(
                    egui::Vec2::new(available_size.x, text_height),
                    egui::TextEdit::multiline(&mut editor.buffer)
                        .font(FontSelection::FontId(FontId::new(
                            16.0,
                            egui::FontFamily::Name("pixel_mplus".into()),
                        )))
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );

                if text_edit_response.changed() {
                    info!("Script editor buffer changed");
                    editor.controls_enabled = false;
                    editor.stage_cleared = false;
                    editor.stage_clear_popup_open = false;
                }

                ui.add_space(8.0);
                if let Some(help) = editor.command_help.as_mut() {
                    let open_label = tr(&localization, "stage-ui-command-help-button");
                    let close_label = tr(&localization, "stage-ui-command-help-close");
                    let button_label = if help.is_open {
                        close_label
                    } else {
                        open_label
                    };
                    if ui.button(button_label.as_str()).clicked() {
                        play_ui_click(&mut commands, &audio);
                        help.is_open = !help.is_open;
                    }

                    if help.is_open {
                        ui.add_space(6.0);
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                let title = tr(&localization, help.title_key);
                                ui.label(RichText::new(title).strong());
                                let intro = tr(&localization, help.intro_key);
                                ui.label(intro);
                                ui.add_space(6.0);
                                for entry in help.entries {
                                    let entry_title = tr(&localization, entry.title_key);
                                    ui.label(RichText::new(entry_title).strong());
                                    let entry_body = tr(&localization, entry.body_key);
                                    ui.label(entry_body);
                                    ui.add_space(4.0);
                                }
                            });
                        });
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
                    play_ui_click(&mut commands, &audio);
                    request_close = true;
                }
            });

        editor.stage_clear_popup_open = popup_open && !request_close;
    }

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
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
        // Remember last commands for UI purposes (summary).
        editor.last_commands.push(command);
    } else {
        // // Program exhausted: stop execution.
        // info!("Script program completed");
        // editor.controls_enabled = false;
        // editor.active_program = None;
    }
}

pub fn tutorial_dialog_for_stage(stage_id: StageId) -> Option<TutorialDialog> {
    match stage_id.0 {
        1 => Some(TutorialDialog::new(
            "stage-ui-tutorial-stage1-title",
            "stage-ui-tutorial-stage1-text",
        )),
        2 => Some(TutorialDialog::new(
            "stage-ui-tutorial-stage2-title",
            "stage-ui-tutorial-stage2-text",
        )),
        3 => Some(TutorialDialog::new(
            "stage-ui-tutorial-stage3-title",
            "stage-ui-tutorial-stage3-text",
        )),
        _ => None,
    }
}

fn command_help_for_stage(_stage_id: StageId) -> Option<CommandHelpDialog> {
    Some(CommandHelpDialog::new(
        "stage-ui-command-help-title",
        "stage-ui-command-help-intro",
        DEFAULT_COMMAND_HELP_ENTRIES,
    ))
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
        text.0.push_str(&panel.hint);
    }
}

pub fn spawn_tutorial_overlay(
    commands: &mut Commands,
    asset_store: &AssetStore,
    localization: &Localization,
    dialog: &TutorialDialog,
) {
    let Some(font) = asset_store.font(FontKey::Default) else {
        warn!("Tutorial overlay: default font is missing");
        return;
    };

    let title = tr(localization, dialog.title_key);
    let body = tr(localization, dialog.body_key);
    let chunks = chunk_tutorial_text(&body);
    if chunks.is_empty() {
        return;
    }
    let hint = tr(localization, "stage-ui-tutorial-next-hint");
    let mut body_value = chunks[0].clone();
    if chunks.len() > 1 {
        body_value.push_str("\n\n");
        body_value.push_str(&hint);
    }

    let mut body_entity = None;
    let panel_entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                right: Val::Px(32.0),
                width: Val::Px(420.0),
                padding: UiRect {
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                    top: Val::Px(12.0),
                    bottom: Val::Px(14.0),
                },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.1, 0.78)),
            ZIndex(5),
            StageTutorialOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont {
                    font: font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.9, 0.65)),
            ));

            let entity = parent
                .spawn((
                    Text::new(body_value),
                    TextFont {
                        font: font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.95, 0.95)),
                ))
                .id();
            body_entity = Some(entity);
        })
        .id();

    if let Some(body_entity) = body_entity {
        commands.entity(panel_entity).insert(TutorialOverlayPanel {
            chunks,
            current_chunk: 0,
            hint,
            body_entity,
        });
    }
}

pub fn handle_tutorial_overlay_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlays: Query<&mut TutorialOverlayPanel>,
    mut texts: Query<&mut Text>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    for mut overlay in &mut overlays {
        if overlay.advance()
            && let Ok(mut text) = texts.get_mut(overlay.body_entity)
        {
            update_overlay_text(&overlay, &mut text);
        }
    }
}
