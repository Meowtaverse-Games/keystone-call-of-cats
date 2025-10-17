use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};
use bevy_egui::{
    EguiContexts,
    egui::{self, load::SizedTexture},
};
use avian2d::prelude::*;

use super::components::*;
use crate::plugins::{
    TiledMapAssets,
    assets_loader::AssetStore,
    design_resolution::{LetterboxOffsets, StageViewport},
};
use crate::scenes::assets::{ImageKey, PLAYER_IDLE_KEYS, PLAYER_RUN_KEYS};

#[derive(Resource, Clone, Copy)]
pub struct StageTileLayout {
    base_tile_size: Vec2,
    map_tile_dimensions: UVec2,
    current_scale: f32,
    last_viewport_size: Vec2,
    origin_offset: Vec2,
}

pub fn setup(
    mut commands: Commands,
    asset_store: Res<AssetStore>,
    tiled_map_assets: Res<TiledMapAssets>,
    windows: Query<&Window, With<PrimaryWindow>>,
    letterbox_offsets: Res<LetterboxOffsets>,
    viewport: Res<StageViewport>,
) {
    let window = match windows.single() {
        Ok(window) => window,
        Err(err) => {
            warn!("Stage setup: primary window unavailable: {err}");
            return;
        }
    };

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

    let tileset = match tiled_map_assets.tilesets().first() {
        Some(tileset) => tileset,
        None => {
            warn!("Stage setup: no tilesets available");
            return;
        }
    };

    let map_tile_dimensions = tiled_map_assets.layers().fold(UVec2::ZERO, |acc, layer| {
        let width = layer.width().max(0) as u32;
        let height = layer.height().max(0) as u32;
        UVec2::new(acc.x.max(width), acc.y.max(height))
    });

    let raw_tile_size = tileset
        .image()
        .map(|image| image.tile_size)
        .unwrap_or(UVec2::new(32, 32));

    let base_tile_size = Vec2::new(raw_tile_size.x.max(1) as f32, raw_tile_size.y.max(1) as f32);

    let mut viewport_size = viewport.size;

    let map_pixel_size = Vec2::new(
        map_tile_dimensions.x as f32 * base_tile_size.x,
        map_tile_dimensions.y as f32 * base_tile_size.y,
    );

    let scale_x = viewport_size.x / map_pixel_size.x;
    let scale_y = viewport_size.y / map_pixel_size.y;
    let scale = scale_x.min(scale_y).max(f32::EPSILON);
    let tile_size = base_tile_size * scale;
    let map_actual_width = map_tile_dimensions.x as f32 * tile_size.x;
    let map_actual_height = map_tile_dimensions.y as f32 * tile_size.y;
    let origin_offset = Vec2::new(-map_actual_width / 2.0 + tile_size.x / 2.0, -map_actual_height / 2.0 + tile_size.y / 2.0);

    commands.insert_resource(StageTileLayout {
        base_tile_size,
        map_tile_dimensions,
        current_scale: scale,
        last_viewport_size: viewport_size,
        origin_offset,
    });

    tiled_map_assets.layers().for_each(|layer| {
        info!("Layer name: {}, type: {:?}", layer.name, layer.layer_type);
        for y in 0..layer.height() {
            for x in 0..layer.width() {
                if let Some(tile) = layer.tile(x as i32, y as i32) {
                    if let Some(tile_sprite) = tileset.atlas_sprite(tile.id) {
                        commands.spawn((
                            StageTile {
                                coord: UVec2::new(x as u32, y as u32),
                            },
                            Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas),
                            Transform::from_xyz(
                                x as f32 * tile_size.x + origin_offset.x,
                                -(y as f32 * tile_size.y + origin_offset.y),
                                0.0,
                            )
                            .with_scale(Vec3::new(scale, scale, 1.0)),
                        ));
                    }
                }
            }
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
        RigidBody::Dynamic,
        Collider::circle(4.5),
        DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
        Transform::from_xyz(0.0, ground_y, 1.0).with_scale(Vec3::splat(4.0)),
    ));
}

pub fn cleanup(
    mut commands: Commands,
    query: Query<Entity, Or<(With<StageBackground>, With<Player>)>>,
    tiles: Query<Entity, With<StageTile>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    for entity in &tiles {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<StageTileLayout>();
}

pub fn update_tiles_on_resize(
    mut resize_events: MessageReader<WindowResized>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    viewport: Res<StageViewport>,
    layout: Option<ResMut<StageTileLayout>>,
    mut tiles: Query<(&StageTile, &mut Transform)>,
) {
    let Ok((window_entity, window)) = windows.single() else {
        for _ in resize_events.read() {}
        return;
    };

    let Some(mut layout) = layout else {
        for _ in resize_events.read() {}
        return;
    };

    let viewport_changed = viewport.is_changed();
    let mut window_resized = false;
    for event in resize_events.read() {
        if event.window == window_entity {
            window_resized = true;
        }
    }

    if !viewport_changed && !window_resized {
        return;
    }

    let mut viewport_size = viewport.size;
    if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
        let window_size = window.resolution.size();
        viewport_size = Vec2::new(window_size.x.max(1.0), window_size.y.max(1.0));
    }

    if (viewport_size.x - layout.last_viewport_size.x).abs() <= f32::EPSILON
        && (viewport_size.y - layout.last_viewport_size.y).abs() <= f32::EPSILON
    {
        return;
    }

    let map_pixel_size = Vec2::new(
        layout.map_tile_dimensions.x as f32 * layout.base_tile_size.x,
        layout.map_tile_dimensions.y as f32 * layout.base_tile_size.y,
    );

    if map_pixel_size.x <= 0.0 || map_pixel_size.y <= 0.0 {
        return;
    }

    let scale_x = viewport_size.x / map_pixel_size.x;
    let scale_y = viewport_size.y / map_pixel_size.y;
    let new_scale = scale_x.min(scale_y).max(f32::EPSILON);

    if (new_scale - layout.current_scale).abs() <= f32::EPSILON {
        layout.last_viewport_size = viewport_size;
        return;
    }

    let tile_size = layout.base_tile_size * new_scale;
    let map_actual_width = layout.map_tile_dimensions.x as f32 * tile_size.x;
    let map_actual_height = layout.map_tile_dimensions.y as f32 * tile_size.y;
    let origin_offset = Vec2::new(-map_actual_width / 2.0 + tile_size.x / 2.0, -map_actual_height / 2.0 + tile_size.y / 2.0);

    for (tile, mut transform) in &mut tiles {
        transform.translation.x = tile.coord.x as f32 * tile_size.x + origin_offset.x;
        transform.translation.y = -(tile.coord.y as f32 * tile_size.y + origin_offset.y);
        transform.scale.x = new_scale;
        transform.scale.y = new_scale;
    }

    layout.current_scale = new_scale;
    layout.last_viewport_size = viewport_size;
    layout.origin_offset = origin_offset;
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
    _window: Single<&mut Window, With<PrimaryWindow>>,
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

    if (letterbox_offsets.left - left).abs() > f32::EPSILON {
        letterbox_offsets.left = left;
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
