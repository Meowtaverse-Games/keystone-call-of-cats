use bevy::{
    math::{URect, UVec2},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    EguiContexts,
    egui::{self, load::SizedTexture},
};

use super::components::{
    CharacterAnimation,
    CharacterMotion,
    StageBackground,
    StageCharacter,
};
use crate::plugins::{assets_loader::AssetStore, design_resolution::LetterboxOffsets};
use crate::scenes::assets::ImageKey;

pub fn setup(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    images: Res<Assets<Image>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    if let Some(texture) = asset_store.image(ImageKey::Spa) {
        commands.spawn((Sprite::from_image(texture.clone()), StageBackground));

        if let Some(image) = images.get(&texture) {
            let image_size = image.size();
            let mut layout = TextureAtlasLayout::new_empty(image_size);

            let frame_size = UVec2::new(25, 30);
            let frame_offset = UVec2::new(6, 480);
            let frame_count = 3usize;

            for index in 0..frame_count {
                let min = frame_offset + UVec2::new(frame_size.x * index as u32, 0);
                let max = min + frame_size;
                layout.add_texture(URect::from_corners(min, max));
            }

            let layout_handle = atlas_layouts.add(layout);

            commands.spawn((
                Sprite::from_atlas_image(texture, TextureAtlas::from(layout_handle)),
                StageCharacter,
                CharacterAnimation {
                    timer: Timer::from_seconds(0.12, TimerMode::Repeating),
                    frames: frame_count,
                },
                CharacterMotion {
                    speed: 90.0,
                    direction: 1.0,
                    min_x: -150.0,
                    max_x: 150.0,
                },
                Transform::from_xyz(0.0, -100.0, 1.0).with_scale(Vec3::splat(4.0)),
            ));
        } else {
            warn!("Stage setup: spa.png image data not found");
        }
    } else {
        warn!("Stage setup: spa.png handle missing");
    }
}

pub fn cleanup(
    mut commands: Commands,
    query: Query<Entity, Or<(With<StageBackground>, With<StageCharacter>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn animate_character(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut CharacterAnimation), With<StageCharacter>>,
) {
    for (mut sprite, mut animation) in &mut query {
        if animation.frames == 0 {
            continue;
        }

        if animation.timer.tick(time.delta()).just_finished() {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = (atlas.index + 1) % animation.frames;
            }
        }
    }
}

pub fn move_character(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut CharacterMotion, &mut Sprite), With<StageCharacter>>,
) {
    for (mut transform, mut motion, mut sprite) in &mut query {
        transform.translation.x += motion.direction * motion.speed * time.delta_secs();

        if transform.translation.x > motion.max_x {
            transform.translation.x = motion.max_x;
            motion.direction = -motion.direction.abs();
        } else if transform.translation.x < motion.min_x {
            transform.translation.x = motion.min_x;
            motion.direction = motion.direction.abs();
        }

        sprite.flip_x = motion.direction < 0.0;
    }
}

pub fn ui(
    mut contexts: EguiContexts,
    asset_store: Res<AssetStore>,
    images: Res<Assets<Image>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
) {
    let logo = texture_handle(&mut contexts, &asset_store, &images, ImageKey::Logo);

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

    let left_logical = left;
    let _left_physical = left_logical * window.scale_factor();

    if (letterbox_offsets.left - left_logical).abs() > f32::EPSILON {
        letterbox_offsets.left = left_logical;
    }

    if letterbox_offsets.right != 0.0 {
        letterbox_offsets.right = 0.0;
    }
}

fn texture_handle(
    contexts: &mut EguiContexts,
    asset_store: &AssetStore,
    images: &Assets<Image>,
    key: ImageKey,
) -> Option<(egui::TextureId, Vec2)> {
    asset_store.image(key).and_then(|handle| {
        images.get(&handle).map(|image| {
            let texture_id = contexts.image_id(&handle).unwrap_or_else(|| {
                contexts.add_image(bevy_egui::EguiTextureHandle::Strong(handle.clone()))
            });

            (texture_id, image.size_f32())
        })
    })
}
