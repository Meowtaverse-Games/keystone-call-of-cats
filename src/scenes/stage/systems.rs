use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{
    EguiContexts,
    egui::{self, load::SizedTexture},
};

use super::components::*;
use crate::plugins::{
    TiledMapAssets, assets_loader::AssetStore, design_resolution::LetterboxOffsets,
};
use crate::scenes::assets::{ImageKey, PLAYER_IDLE_KEYS, PLAYER_RUN_KEYS};

pub fn setup(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    tiled_map_assets: Res<TiledMapAssets>,
) {
    let idle_frames: Vec<Handle<Image>> = PLAYER_IDLE_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();
    let run_frames: Vec<Handle<Image>> = PLAYER_RUN_KEYS
        .iter()
        .filter_map(|key| asset_store.image(*key))
        .collect();

    if idle_frames.is_empty() && run_frames.is_empty() {
        warn!("Stage setup: no player animation frames found");
        return;
    }

    if idle_frames.is_empty() {
        warn!("Stage setup: Idle animation frames missing; falling back to run frames");
    }

    if run_frames.is_empty() {
        warn!("Stage setup: Run animation frames missing; player will stay idle");
    }

    let clips = PlayerAnimationClips {
        idle: idle_frames,
        run: run_frames,
    };

    let initial_state = if clips.idle.is_empty() {
        PlayerAnimationState::Run
    } else {
        PlayerAnimationState::Idle
    };

    let initial_frame = clips
        .frames(initial_state)
        .first()
        .cloned()
        .or_else(|| clips.frames(PlayerAnimationState::Run).first().cloned())
        .or_else(|| clips.frames(PlayerAnimationState::Idle).first().cloned());

    let Some(initial_frame) = initial_frame else {
        warn!("Stage setup: could not determine an initial player sprite");
        return;
    };

    tiled_map_assets.layers().for_each(|layer| {
        info!("Layer name: {}, type: {:?}", layer.name, layer.layer_type);
        for y in 0..layer.height() {
            for x in 0..layer.width() {
                if let Some(tile) = layer.tile(x as i32, y as i32) {
                    info!("  Tile at ({}, {}): id={}, collision={:?}", x, y, tile.id, tile.collision);
                }
            }
        }
    });

    tiled_map_assets.tilesets().iter().for_each(|tileset| {
        info!("Tileset: {}", tileset.name());
        if let (Some(image), Some(tile_sprite)) = (tileset.image(), tileset.atlas_sprite(0)) {
            let mut sprite = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
            sprite.custom_size =
                Some(Vec2::new(image.tile_size.x as f32, image.tile_size.y as f32) * 4.0);

            commands.spawn((
                sprite,
                Transform::from_xyz(-320.0, -160.0, 0.0),
                Name::new(format!(
                    "{}::tile0 ({})",
                    tileset.name(),
                    image.path.as_str()
                )),
            ));
        }
    });

    let ground_y = -100.0;

    commands.spawn((
        Sprite::from_image(initial_frame),
        Player,
        PlayerAnimation {
            timer: Timer::from_seconds(0.12, TimerMode::Repeating),
            clips,
            state: initial_state,
            frame_index: 0,
        },
        PlayerMotion {
            speed: 90.0,
            direction: 1.0,
            min_x: -150.0,
            max_x: 150.0,
            is_moving: matches!(initial_state, PlayerAnimationState::Run),
            vertical_velocity: 0.0,
            gravity: -600.0,
            jump_speed: 280.0,
            ground_y,
            is_jumping: false,
        },
        Transform::from_xyz(0.0, ground_y, 1.0).with_scale(Vec3::splat(4.0)),
    ));
}

pub fn cleanup(
    mut commands: Commands,
    query: Query<Entity, Or<(With<StageBackground>, With<Player>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn animate_character(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation, &PlayerMotion), With<Player>>,
) {
    for (mut sprite, mut animation, motion) in &mut query {
        let desired_state = if motion.is_moving {
            PlayerAnimationState::Run
        } else {
            PlayerAnimationState::Idle
        };

        if animation.state != desired_state && !animation.clips.frames(desired_state).is_empty() {
            animation.state = desired_state;
            animation.frame_index = 0;
            animation.timer.reset();

            if let Some(handle) = animation.current_frames().first() {
                sprite.image = handle.clone();
            }
        }

        let frame_count = animation.current_frames().len();
        if frame_count == 0 {
            continue;
        }

        if animation.timer.tick(time.delta()).just_finished() {
            animation.frame_index = (animation.frame_index + 1) % frame_count;
            if let Some(handle) = animation.current_frames().get(animation.frame_index) {
                sprite.image = handle.clone();
            }
        }
    }
}

pub fn move_character(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut PlayerMotion, &mut Sprite), With<Player>>,
) {
    for (mut transform, mut motion, mut sprite) in &mut query {
        let mut input_direction: f32 = 0.0;

        if keyboard_input.pressed(KeyCode::ArrowRight) {
            input_direction += 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            input_direction -= 1.0;
        }

        let mut moved = false;

        if input_direction.abs() > f32::EPSILON {
            let direction = input_direction.signum();
            let delta = direction * motion.speed * time.delta_secs();
            let target_x = (transform.translation.x + delta).clamp(motion.min_x, motion.max_x);

            moved = (target_x - transform.translation.x).abs() > f32::EPSILON;
            transform.translation.x = target_x;
            motion.direction = direction;
        }

        if keyboard_input.just_pressed(KeyCode::Space) && !motion.is_jumping {
            motion.is_jumping = true;
            motion.vertical_velocity = motion.jump_speed;
        }

        if motion.is_jumping || transform.translation.y > motion.ground_y {
            motion.vertical_velocity += motion.gravity * time.delta_secs();
            transform.translation.y += motion.vertical_velocity * time.delta_secs();

            if transform.translation.y <= motion.ground_y {
                transform.translation.y = motion.ground_y;
                motion.vertical_velocity = 0.0;
                motion.is_jumping = false;
            }
        }

        motion.is_moving = moved;
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
