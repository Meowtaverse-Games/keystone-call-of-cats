use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, FontId, FontSelection},
};

use crate::{
    core::boundary::ScriptCommand,
    plugins::{
        design_resolution::LetterboxOffsets,
        script::{Language, ScriptExecutor},
    },
    scenes::stage::systems::StoneCommandMessage,
};

#[derive(Resource, Default)]
pub struct ScriptEditorState {
    pub buffer: String,
    pub last_action: Option<EditorMenuAction>,
    pub last_run_feedback: Option<String>,
    pub last_commands: Vec<ScriptCommand>,
}

impl ScriptEditorState {
    fn apply_action(&mut self, action: EditorMenuAction) {
        self.last_action = Some(action);
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

    fn label(self) -> &'static str {
        match self {
            Self::LoadExample => "Load",
            Self::SaveBuffer => "Save",
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

    fn status_text(self) -> &'static str {
        match self {
            Self::LoadExample => "メニュー「ロード（F1）」を選択しました。",
            Self::SaveBuffer => "メニュー「セーブ（F2）」を選択しました。",
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
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut action_from_keys = None;
    ctx.input(|input| {
        for action in EditorMenuAction::ALL {
            if input.key_pressed(action.key()) {
                action_from_keys = Some(action);
                break;
            }
        }
    });

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
                let mut pending_action = action_from_keys;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    for action in EditorMenuAction::ALL {
                        let label = format!("{} ({})", action.label(), action.key_text());
                        if ui.button(label).clicked() {
                            pending_action = Some(action);
                        }
                    }
                });

                if let Some(action) = pending_action {
                    if action == EditorMenuAction::RunScript {
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
                            }
                            Err(err) => {
                                editor.last_commands.clear();
                                editor.last_run_feedback = Some(err.to_string());
                                warn!("Failed to execute script: {}", err);
                            }
                        }
                    }
                    editor.apply_action(action);
                }

                if let Some(action) = editor.last_action {
                    info!("Editor action: {:?}", action);
                    ui.label(action.status_text());
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

                ui.add_sized(
                    available_size,
                    egui::TextEdit::multiline(&mut editor.buffer)
                        .font(FontSelection::FontId(FontId::new(
                            16.0,
                            egui::FontFamily::Name("pixel_mplus".into()),
                        )))
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            });
        })
        .response
        .rect
        .width()
        .clamp(min_width, max_width);

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
        letterbox_offsets.left = left;
    }
}
