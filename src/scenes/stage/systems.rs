use bevy_egui::{
    egui, EguiContexts,
};

pub fn ui(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let left = egui::SidePanel::left("left")
        .resizable(true)
        .default_width(200.0)
        .min_width(100.0)
        .max_width(300.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(40, 40, 60),
            inner_margin: egui::Margin::same(10),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 150)),
            ..Default::default()
        });
    left.show(ctx, |ui| {
        ui.label("Loading...");

        ui.separator();
    });
}
