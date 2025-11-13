use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Event, FontId, FontSelection},
};
use bevy_fluent::prelude::Localization;

use crate::{
    resources::{
        design_resolution::LetterboxOffsets,
        game_state::GameState,
        script_engine::{Language, ScriptExecutor},
        stage_catalog::StageId,
    },
    scenes::stage::systems::StoneCommandMessage,
    util::{
        localization::{script_error_message, tr, tr_with_args},
        script_types::ScriptCommand,
    },
};

#[derive(Clone, Debug)]
pub struct TutorialDialog {
    title_key: &'static str,
    line_keys: &'static [&'static str],
    is_open: bool,
}

impl TutorialDialog {
    fn new(title_key: &'static str, line_keys: &'static [&'static str]) -> Self {
        Self {
            title_key,
            line_keys,
            is_open: true,
        }
    }
}

#[derive(Resource, Default)]
pub struct ScriptEditorState {
    pub buffer: String,
    pub last_action: Option<EditorMenuAction>,
    pub last_action_was_running: bool,
    pub last_run_feedback: Option<String>,
    pub last_commands: Vec<ScriptCommand>,
    pub controls_enabled: bool,
    pub pending_player_reset: bool,
    pub stage_cleared: bool,
    pub stage_clear_popup_open: bool,
    pub tutorial_dialog: Option<TutorialDialog>,
}

impl ScriptEditorState {
    fn apply_action(&mut self, action: EditorMenuAction, was_running: bool) {
        self.last_action = Some(action);
        self.last_action_was_running = was_running;
    }

    pub fn set_tutorial_for_stage(&mut self, stage_id: StageId) {
        self.tutorial_dialog = tutorial_dialog_for_stage(stage_id);
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
            Self::LoadExample => "stage-ui-menu.load",
            Self::SaveBuffer => "stage-ui-menu.save",
            Self::RunScript if is_running => "stage-ui-menu.stop",
            Self::RunScript => "stage-ui-menu.run",
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
            Self::LoadExample => "stage-ui-status.load",
            Self::SaveBuffer => "stage-ui-status.save",
            Self::RunScript if was_running => "stage-ui-status.stop",
            Self::RunScript => "stage-ui-status.run",
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
    commands.insert_resource(editor_state);
}

pub fn ui(
    mut contexts: EguiContexts,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
    mut editor: ResMut<ScriptEditorState>,
    script_executor: Res<ScriptExecutor>,
    localization: Res<Localization>,
    mut stone_writer: MessageWriter<StoneCommandMessage>,
    mut next_state: ResMut<NextState<GameState>>,
) {
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
                    editor.controls_enabled = false;
                    editor.pending_player_reset = false;
                    editor.stage_cleared = false;
                    editor.stage_clear_popup_open = false;
                    next_state.set(GameState::SelectStage);
                }

                ui.separator();

                let mut pending_action = action_from_keys;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    for action in EditorMenuAction::ALL {
                        let button_label =
                            tr(&localization, action.label_key(editor.controls_enabled));
                        let label = format!("{} ({})", button_label, action.key_text());
                        if ui.button(label).clicked() {
                            pending_action = Some(action);
                        }
                    }
                });

                if let Some(action) = pending_action {
                    let was_running = editor.controls_enabled;
                    match action {
                        EditorMenuAction::RunScript => {
                            if was_running {
                                editor.controls_enabled = false;
                                editor.pending_player_reset = true;
                                editor.last_run_feedback =
                                    Some(tr(&localization, "stage-ui-feedback.stopped"));
                                editor.stage_cleared = false;
                                editor.stage_clear_popup_open = false;
                            } else {
                                match script_executor.run(Language::Rhai, &editor.buffer) {
                                    Ok(commands) => {
                                        let summary = if commands.is_empty() {
                                            tr(&localization, "stage-ui-feedback.no-commands")
                                        } else {
                                            let command_summary = summarize_commands(&commands);
                                            let count = commands.len().to_string();
                                            tr_with_args(
                                                &localization,
                                                "stage-ui-feedback.commands",
                                                &[
                                                    ("count", count.as_str()),
                                                    ("summary", command_summary.as_str()),
                                                ],
                                            )
                                        };
                                        stone_writer.write(StoneCommandMessage {
                                            commands: commands.clone(),
                                        });
                                        editor.last_commands = commands;
                                        editor.last_run_feedback = Some(summary);
                                        editor.controls_enabled = true;
                                        editor.pending_player_reset = true;
                                        editor.stage_cleared = false;
                                        editor.stage_clear_popup_open = false;
                                    }
                                    Err(err) => {
                                        editor.last_commands.clear();
                                        editor.last_run_feedback =
                                            Some(script_error_message(&localization, &err));
                                        editor.controls_enabled = false;
                                        editor.pending_player_reset = false;
                                        editor.stage_cleared = false;
                                        editor.stage_clear_popup_open = false;
                                        warn!("Failed to execute script: {}", err);
                                    }
                                }
                            }
                        }
                        EditorMenuAction::LoadExample => {
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
                        "stage-ui-commands.list",
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

                let text_edit_response = ui.add_sized(
                    available_size,
                    egui::TextEdit::multiline(&mut editor.buffer)
                        .font(FontSelection::FontId(FontId::new(
                            16.0,
                            egui::FontFamily::Name("pixel_mplus".into()),
                        )))
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );

                if text_edit_response.changed() {
                    editor.controls_enabled = false;
                    editor.stage_cleared = false;
                    editor.stage_clear_popup_open = false;
                }
            });
        })
        .response
        .rect
        .width()
        .clamp(min_width, max_width);

    if let Some(tutorial) = editor.tutorial_dialog.as_mut()
        && tutorial.is_open
    {
        let mut open = tutorial.is_open;
        let mut request_close = false;
        let window_width = if screen_width.is_finite() && screen_width > 0.0 {
            screen_width.min(420.0)
        } else {
            320.0
        };
        let title = tr(&localization, tutorial.title_key);
        egui::Window::new(title)
            .anchor(Align2::CENTER_TOP, egui::Vec2::new(0.0, 20.0))
            .resizable(false)
            .collapsible(false)
            .default_width(window_width)
            .open(&mut open)
            .show(ctx, |ui| {
                for line_key in tutorial.line_keys {
                    let line = tr(&localization, line_key);
                    ui.label(line);
                }
                ui.add_space(8.0);
                let controls = tr(&localization, "stage-ui-tutorial.controls-hint");
                ui.label(controls);
                ui.add_space(12.0);
                let ok = tr(&localization, "stage-ui-tutorial.ok");
                if ui.button(ok.as_str()).clicked() {
                    request_close = true;
                }
            });

        tutorial.is_open = open && !request_close;
    }

    if editor.stage_clear_popup_open {
        let mut popup_open = editor.stage_clear_popup_open;
        let mut request_close = false;
        let window_title = tr(&localization, "stage-ui-clear.window-title");
        egui::Window::new(window_title)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .open(&mut popup_open)
            .show(ctx, |ui| {
                let heading = tr(&localization, "stage-ui-clear.heading");
                ui.heading(heading);
                ui.add_space(8.0);
                let body = tr(&localization, "stage-ui-clear.body");
                ui.label(body);
                ui.add_space(12.0);
                let ok = tr(&localization, "stage-ui-clear.ok");
                if ui.button(ok.as_str()).clicked() {
                    request_close = true;
                }
            });

        editor.stage_clear_popup_open = popup_open && !request_close;
    }

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
        letterbox_offsets.left = left;
    }
}

fn tutorial_dialog_for_stage(stage_id: StageId) -> Option<TutorialDialog> {
    match stage_id.0 {
        1 => Some(TutorialDialog::new(
            "stage-ui-tutorial.stage1.title",
            &[
                "stage-ui-tutorial.stage1.line1",
                "stage-ui-tutorial.stage1.line2",
                "stage-ui-tutorial.stage1.line3",
            ],
        )),
        2 => Some(TutorialDialog::new(
            "stage-ui-tutorial.stage2.title",
            &[
                "stage-ui-tutorial.stage2.line1",
                "stage-ui-tutorial.stage2.line2",
                "stage-ui-tutorial.stage2.line3",
            ],
        )),
        3 => Some(TutorialDialog::new(
            "stage-ui-tutorial.stage3.title",
            &[
                "stage-ui-tutorial.stage3.line1",
                "stage-ui-tutorial.stage3.line2",
                "stage-ui-tutorial.stage3.line3",
            ],
        )),
        _ => None,
    }
}
