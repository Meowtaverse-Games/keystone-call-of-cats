use bevy::{input::ButtonInput, prelude::*};
use bevy_egui::{
    EguiContexts,
    egui::{self, Color32, Frame, Id, LayerId, Margin, Order, RichText, Sense, Vec2},
};
use bevy_fluent::prelude::Localization;

use crate::{
    resources::{script_engine::Language, settings::GameSettings},
    scenes::audio::{AudioHandles, play_ui_click},
    util::localization::tr,
};

const LABEL_COLOR: Color32 = Color32::from_rgb(0xff, 0xf1, 0xf1);
const SLIDER_EMPTY: Color32 = Color32::from_rgb(0xfe, 0xfa, 0xeb);
const SLIDER_FILL: Color32 = Color32::from_rgb(0xf2, 0x4c, 0x86);
const SLIDER_KNOB: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
const SLIDER_KNOB_RING: Color32 = Color32::from_rgb(0xff, 0x45, 0x7f);

#[derive(Resource, Default)]
pub struct OptionsOverlayState {
    pub open: bool,
}

pub fn handle_overlay_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<OptionsOverlayState>,
) {
    if !overlay.open {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        overlay.open = false;
    }
}

pub fn options_overlay_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut settings: ResMut<GameSettings>,
    localization: Res<Localization>,
    audio: Res<AudioHandles>,
    mut overlay: ResMut<OptionsOverlayState>,
) {
    if !overlay.open {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let screen_rect = ctx.content_rect();
    let margin = Vec2::new(screen_rect.width() * 0.15, screen_rect.height() * 0.12);
    let panel_rect = screen_rect.shrink2(margin);

    ctx.layer_painter(LayerId::new(
        Order::Background,
        Id::new("options-overlay-dim"),
    ))
    .rect_filled(
        screen_rect,
        0.0,
        Color32::from_rgba_unmultiplied(8, 12, 28, 180),
    );

    egui::Area::new(Id::new("stage-select-options-overlay"))
        .order(Order::Foreground)
        .fixed_pos(panel_rect.min)
        .show(ctx, |ui| {
            ui.set_width(panel_rect.width());
            ui.set_height(panel_rect.height());
            Frame::new()
                .fill(Color32::from_rgb(0x10, 0x18, 0x2a))
                .inner_margin(Margin::symmetric(36, 28))
                .show(ui, |ui| {
                    ui.set_width(panel_rect.width() - 72.0);
                    ui.set_height(panel_rect.height() - 56.0);
                    draw_contents(
                        ui,
                        &mut commands,
                        &mut settings,
                        &localization,
                        &audio,
                        &mut overlay,
                    );
                });
        });
}

fn draw_contents(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    settings: &mut GameSettings,
    localization: &Localization,
    audio: &AudioHandles,
    overlay: &mut OptionsOverlayState,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr(localization, "options-title"))
                .size(36.0)
                .color(LABEL_COLOR)
                .strong(),
        );
        ui.add_space(32.0);

        let mut master = settings.master_volume_percent();
        let mut sfx = settings.sfx_volume_percent();
        let mut music = settings.music_volume_percent();

        if volume_slider(ui, tr(localization, "options-volume-master"), &mut master) {
            settings.set_master_volume_percent(master);
        }
        if volume_slider(ui, tr(localization, "options-volume-sfx"), &mut sfx) {
            settings.set_sfx_volume_percent(sfx);
        }
        if volume_slider(ui, tr(localization, "options-volume-music"), &mut music) {
            settings.set_music_volume_percent(music);
        }

        ui.add_space(8.0);
        fullscreen_toggle(ui, settings, localization);
        ui.add_space(12.0);
        language_selector(ui, settings, localization);

        ui.add_space(32.0);
        let button = egui::Button::new(
            RichText::new(tr(localization, "options-button-back"))
                .color(LABEL_COLOR)
                .size(22.0),
        )
        .min_size(Vec2::new(200.0, 46.0))
        .fill(Color32::from_rgb(0x29, 0x1c, 0x33));

        if ui.add(button).clicked() {
            play_ui_click(commands, audio, settings);
            overlay.open = false;
        }
    });
}

fn volume_slider(ui: &mut egui::Ui, label: String, value: &mut f32) -> bool {
    *value = value.clamp(0.0, 100.0);
    let mut changed = false;

    ui.label(
        RichText::new(format!("{label} : {:02}", value.round() as i32))
            .size(28.0)
            .color(LABEL_COLOR),
    );

    let width = ui.available_width().max(320.0);
    let height = 24.0;
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, height + 8.0), Sense::click_and_drag());

    let slider_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - height * 0.5),
        Vec2::new(rect.width(), height),
    );

    if (response.dragged() || response.clicked()) && response.interact_pointer_pos().is_some() {
        let pointer = response.interact_pointer_pos().unwrap();
        let t = ((pointer.x - slider_rect.left()) / slider_rect.width()).clamp(0.0, 1.0);
        let new_value = (t * 100.0).clamp(0.0, 100.0);
        if (new_value - *value).abs() > f32::EPSILON {
            *value = new_value;
            changed = true;
            response.mark_changed();
        }
    }

    let painter = ui.painter();
    painter.rect_filled(slider_rect, 2.0, SLIDER_EMPTY);
    let filled_width = slider_rect.width() * (*value / 100.0);
    let filled_rect = egui::Rect::from_min_size(
        slider_rect.left_top(),
        Vec2::new(filled_width, slider_rect.height()),
    );
    painter.rect_filled(filled_rect, 2.0, SLIDER_FILL);

    let knob_center = egui::pos2(slider_rect.left() + filled_width, slider_rect.center().y);
    let knob_radius = slider_rect.height() * 0.5;
    painter.circle_filled(knob_center, knob_radius, SLIDER_KNOB);
    painter.circle_stroke(
        knob_center,
        knob_radius,
        egui::Stroke::new(3.0, SLIDER_KNOB_RING),
    );

    ui.add_space(16.0);
    changed
}

fn fullscreen_toggle(ui: &mut egui::Ui, settings: &mut GameSettings, localization: &Localization) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(tr(localization, "options-fullscreen-label"))
                .size(22.0)
                .color(LABEL_COLOR),
        );
        ui.add_space(12.0);
        let label = if settings.fullscreen {
            tr(localization, "options-fullscreen-on")
        } else {
            tr(localization, "options-fullscreen-off")
        };
        let response = ui.add(
            egui::Button::new(
                RichText::new(label)
                    .color(Color32::from_rgb(0x12, 0x0c, 0x1c))
                    .size(20.0),
            )
            .min_size(Vec2::new(96.0, 34.0))
            .fill(Color32::from_rgb(0xff, 0xf6, 0xd8)),
        );
        if response.clicked() {
            settings.fullscreen = !settings.fullscreen;
        }
    });
}

fn language_selector(ui: &mut egui::Ui, settings: &mut GameSettings, localization: &Localization) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(tr(localization, "options-language-label"))
                .size(22.0)
                .color(LABEL_COLOR),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (language, key) in [
                (Language::Rhai, "options-language-rhai"),
                (Language::Keystone, "options-language-keystone"),
            ] {
                let selected = settings.script_language == language;
                let mut button =
                    egui::Button::new(RichText::new(tr(localization, key)).size(20.0).color(
                        if selected {
                            Color32::from_rgb(0x12, 0x0c, 0x1c)
                        } else {
                            LABEL_COLOR
                        },
                    ))
                    .min_size(Vec2::new(140.0, 34.0));
                if selected {
                    button = button.fill(Color32::from_rgb(0xf8, 0xd3, 0xec));
                } else {
                    button = button.fill(Color32::from_rgb(0x1f, 0x1a, 0x2a));
                }

                if ui.add(button).clicked() {
                    settings.script_language = language;
                }
                ui.add_space(12.0);
            }
        });
    });
}
