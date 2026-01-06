use avian2d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct AnimatedObstacle {
    pub animation_timer: Timer,
    pub lifetime_timer: Timer,
    pub loop_frames: Vec<usize>,
    pub vanish_frames: Vec<usize>,
    pub current_step: usize,
    pub is_vanishing: bool,
}

pub fn spawn_obstacle(
    commands: &mut Commands,
    stage_root: Entity,
    asset_server: &AssetServer,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    (x, y, scale): (f32, f32, f32),
) {
    let tile_width = 16;
    let tile_height = 16;
    let spacing = 3;
    let margin = 0;
    let columns = 17;
    let tile_count = 561;

    let texture: Handle<Image> = asset_server.load("images/spa.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(tile_width, tile_height),
        columns,
        tile_count,
        Some(UVec2::splat(spacing)),
        Some(UVec2::splat(margin)),
    );
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    // Frame indices based on requirement:
    // First 4 frames: Loop
    // Last 4 frames: Vanish
    let all_indices = vec![195, 196, 197, 198, 212, 213, 214, 215];
    let loop_frames = vec![195, 196, 197, 198, 197, 196];
    let vanish_frames = all_indices[4..8].to_vec();

    let mut rng = rand::rng();
    // Random duration between 2 and 5 seconds (adjust as needed)
    let duration = rng.random_range(2.0..5.0);

    let obstacle_entity = commands
        .spawn((
            Sprite {
                image: texture,
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout,
                    index: loop_frames[0],
                }),
                ..default()
            },
            Transform::from_xyz(x, y, 10.0).with_scale(Vec3::splat(scale)),
            AnimatedObstacle {
                animation_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
                lifetime_timer: Timer::from_seconds(duration, TimerMode::Once),
                loop_frames,
                vanish_frames,
                current_step: 0,
                is_vanishing: false,
            },
            RigidBody::Static,
            Collider::rectangle(tile_width as f32, tile_height as f32),
        ))
        .id();
    commands.entity(stage_root).add_child(obstacle_entity);
}

pub fn animate_obstacle(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AnimatedObstacle, &mut Sprite)>,
) {
    for (entity, mut obstacle, mut sprite) in &mut query {
        obstacle.animation_timer.tick(time.delta());
        obstacle.lifetime_timer.tick(time.delta());

        // Check if lifetime expired to switch to vanishing mode
        if !obstacle.is_vanishing && obstacle.lifetime_timer.is_finished() {
            obstacle.is_vanishing = true;
            obstacle.current_step = 0; // Start vanish animation from beginning

            // Remove collision when vanishing starts
            commands.entity(entity).remove::<Collider>();
            commands.entity(entity).remove::<RigidBody>();

            // Force update display immediately
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = obstacle.vanish_frames[0];
            }
        }

        if obstacle.animation_timer.just_finished() {
            if obstacle.is_vanishing {
                obstacle.current_step += 1;
                if obstacle.current_step >= obstacle.vanish_frames.len() {
                    commands.entity(entity).despawn();
                } else if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = obstacle.vanish_frames[obstacle.current_step];
                }
            } else {
                // Looping mode
                obstacle.current_step = (obstacle.current_step + 1) % obstacle.loop_frames.len();
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = obstacle.loop_frames[obstacle.current_step];
                }
            }
        }
    }
}
