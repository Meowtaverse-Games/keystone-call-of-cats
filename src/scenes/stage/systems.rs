use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use bevy_egui::{
    EguiContexts,
    egui::{self, load::SizedTexture},
};

use super::components::StageBackground;
use crate::plugins::{assets_loader::AssetStore, design_resolution::MainCamera};
use crate::scenes::assets::ImageKey;

pub fn setup(mut commands: Commands, asset_store: Res<AssetStore>) {
    if let Some(texture) = asset_store.image(ImageKey::Spa) {
        commands.spawn((Sprite::from_image(texture), StageBackground));
    } else {
        // warn!("Stage setup: spa.png handle missing");
    }
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<StageBackground>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn ui(
    mut contexts: EguiContexts,
    asset_store: Res<AssetStore>,
    images: Res<Assets<Image>>,
    mut camera: Single<&mut Camera, With<MainCamera>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let logo = texture_handle(&mut contexts, &asset_store, &images, ImageKey::Logo);

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut left = egui::SidePanel::left("stage-left")
        .resizable(true)
        .default_width(200.0)
        .min_width(100.0)
        .max_width(300.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(255, 255, 255),
            inner_margin: egui::Margin::same(10),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 150)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some((texture_id, size)) = logo {
                            ui.image(SizedTexture::new(texture_id, [size.x, size.y]));
                        } else {
                            ui.label("Loading...");
                        }
                    });
                });
        })
        .response
        .rect
        .width();

    left *= window.scale_factor();

    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(left as u32, 0),
        physical_size: UVec2::new(
            (window.physical_width() as f32 - left) as u32,
            window.physical_height()),
        ..default()
    });

    println!(
        "window: {:?}, left panel: {}\n",
        window.physical_size(),
        left
    );
}

fn texture_handle(
    contexts: &mut EguiContexts,
    asset_store: &AssetStore,
    images: &Assets<Image>,
    key: ImageKey,
) -> Option<(egui::TextureId, Vec2)> {
    asset_store.image(key).and_then(|handle| {
        images.get(&handle).map(|image| {
            let texture_id = contexts
                .image_id(&handle)
                .unwrap_or_else(|| contexts.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone())));

            (texture_id, image.size_f32())
        })
    })
}
