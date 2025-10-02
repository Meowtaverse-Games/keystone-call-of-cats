use bevy::prelude::{Assets, Image, Res};
use bevy_egui::{
    EguiContexts,
    egui::{self, load::SizedTexture},
};

use crate::plugins::assets_loader::AssetStore;
use crate::scenes::assets::ImageKey;

pub fn ui(mut contexts: EguiContexts, asset_store: Res<AssetStore>, images: Res<Assets<Image>>) {
    let texture = asset_store.image(ImageKey::Logo).and_then(|handle| {
        images.get(&handle).map(|image| {
            let texture_id = contexts
                .image_id(&handle)
                .unwrap_or_else(|| contexts.add_image(handle.clone_weak()));

            (texture_id, image.size_f32())
        })
    });

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let left = egui::SidePanel::left("stage-left")
        .resizable(true)
        .default_width(200.0)
        .min_width(100.0)
        .max_width(300.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(255, 255, 255),
            inner_margin: egui::Margin::same(10),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 150)),
            ..Default::default()
        });
    left.show(ctx, |ui| {
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some((texture_id, size)) = texture {
                        ui.image(SizedTexture::new(texture_id, size.to_array()));
                    } else {
                        ui.label("Loading...");
                    }
                });
            });
    });
}
