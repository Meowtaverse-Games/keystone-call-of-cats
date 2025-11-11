use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Event, FontId, FontSelection},
};

use crate::{
    resources::{
        design_resolution::LetterboxOffsets,
        game_state::GameState,
        script_engine::{Language, ScriptExecutor},
    },
    scenes::stage::systems::StoneCommandMessage,
    util::script_types::ScriptCommand,
};

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
}

impl ScriptEditorState {
    fn apply_action(&mut self, action: EditorMenuAction, was_running: bool) {
        self.last_action = Some(action);
        self.last_action_was_running = was_running;
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

    fn label(self, is_running: bool) -> &'static str {
        match self {
            Self::LoadExample => "Load",
            Self::SaveBuffer => "Save",
            Self::RunScript if is_running => "Stop",
            Self::RunScript => "Run",
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

    fn status_text(self, was_running: bool) -> &'static str {
        match self {
            Self::LoadExample => "メニュー「ロード（F1）」を選択しました。",
            Self::SaveBuffer => "メニュー「セーブ（F2）」を選択しました。",
            Self::RunScript if was_running => "メニュー「停止（F3）」を選択しました。",
            Self::RunScript => "メニュー「実行（F3）」を選択しました。",
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

pub fn init_editor_state(commands: &mut Commands) {
    commands.insert_resource(ScriptEditorState {
        buffer: String::from(
            "move(\"left\");\n\
             sleep(1.0);\n\
             move(\"right\");\n\
             sleep(1.0);\n\
             for i in 1..=2 {\n  move(\"up\");\n  sleep(0.5);\n\
             }\n",
        ),
        ..default()
    });
}

pub fn ui(
    mut contexts: EguiContexts,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
    mut editor: ResMut<ScriptEditorState>,
    script_executor: Res<ScriptExecutor>,
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
                if ui.button("タイトルに戻る").clicked() {
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
                        let label = format!(
                            "{} ({})",
                            action.label(editor.controls_enabled),
                            action.key_text()
                        );
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
                                editor.last_run_feedback = Some("実行を停止しました。".to_string());
                                editor.stage_cleared = false;
                                editor.stage_clear_popup_open = false;
                            } else {
                                match script_executor.run(Language::Rhai, &editor.buffer) {
                                    Ok(commands) => {
                                        let summary = if commands.is_empty() {
                                            "命令は返されませんでした。".to_string()
                                        } else {
                                            format!(
                                                "{}件の命令: {}",
                                                commands.len(),
                                                summarize_commands(&commands)
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
                                        editor.last_run_feedback = Some(err.to_string());
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
                    ui.label(action.status_text(editor.last_action_was_running));
                    editor.last_action = None;
                }

                if let Some(feedback) = &editor.last_run_feedback {
                    ui.label(feedback);
                }

                if !editor.last_commands.is_empty() {
                    ui.label(format!(
                        "命令: {}",
                        summarize_commands(&editor.last_commands)
                    ));
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

    if editor.stage_clear_popup_open {
        let mut popup_open = editor.stage_clear_popup_open;
        let mut request_close = false;
        egui::Window::new("ステージクリア!")
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .open(&mut popup_open)
            .show(ctx, |ui| {
                ui.heading("ゴールに到達しました。");
                ui.add_space(8.0);
                ui.label("次の挑戦へ進む前に少し休憩しましょう。");
                ui.add_space(12.0);
                if ui.button("OK").clicked() {
                    request_close = true;
                }
            });

        editor.stage_clear_popup_open = popup_open && !request_close;
    }

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
        letterbox_offsets.left = left;
    }
}
