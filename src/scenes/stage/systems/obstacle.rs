use crate::scenes::stage::systems::ui::ScriptEditorState;
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
    pub collider_size: Vec2,
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
    let collider_size = Vec2::new(tile_width as f32, tile_height as f32);

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
                collider_size,
            },
            RigidBody::Kinematic,
            Collider::rectangle(collider_size.x, collider_size.y),
        ))
        .id();
    commands.entity(stage_root).add_child(obstacle_entity);
}

pub fn animate_obstacle(
    mut commands: Commands,
    time: Res<Time>,
    editor_state: Option<Res<ScriptEditorState>>,
    mut query: Query<(
        Entity,
        &mut AnimatedObstacle,
        &mut Sprite,
        &mut Visibility,
        Option<&Collider>,
    )>,
) {
    let is_playing = editor_state.map(|s| s.controls_enabled).unwrap_or(false);

    for (entity, mut obstacle, mut sprite, mut visibility, collider) in &mut query {
        if !is_playing {
            // Edit Mode: Reset and Loop
            if *visibility == Visibility::Hidden || obstacle.is_vanishing {
                // Reset to initial state
                obstacle.is_vanishing = false;
                obstacle.current_step = 0;
                *visibility = Visibility::Visible;

                // Reset timer with new random duration
                let mut rng = rand::rng();
                let duration = rng.random_range(2.0..5.0);
                obstacle
                    .lifetime_timer
                    .set_duration(std::time::Duration::from_secs_f32(duration));
                obstacle.lifetime_timer.reset();

                // Restore collision if missing.
                // Note: We keeping RigidBody always, only toggling Collider preventing avian2d panic.
                if collider.is_none() {
                    commands.entity(entity).insert(Collider::rectangle(
                        obstacle.collider_size.x,
                        obstacle.collider_size.y,
                    ));
                }
            }

            // Always loop animation in edit mode (ignoring lifetime timer)
            obstacle.animation_timer.tick(time.delta());
            if obstacle.animation_timer.just_finished() {
                obstacle.current_step = (obstacle.current_step + 1) % obstacle.loop_frames.len();
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = obstacle.loop_frames[obstacle.current_step];
                }
            }
        } else {
            // Play Mode
            if *visibility != Visibility::Hidden {
                obstacle.animation_timer.tick(time.delta());
                obstacle.lifetime_timer.tick(time.delta());

                // Timer expired -> Start Vanish
                if !obstacle.is_vanishing && obstacle.lifetime_timer.is_finished() {
                    obstacle.is_vanishing = true;
                    obstacle.current_step = 0;

                    // Remove collision immediately (only Collider)
                    commands.entity(entity).remove::<Collider>();

                    if let Some(atlas) = &mut sprite.texture_atlas {
                        atlas.index = obstacle.vanish_frames[0];
                    }
                }

                if obstacle.animation_timer.just_finished() {
                    if obstacle.is_vanishing {
                        obstacle.current_step += 1;
                        if obstacle.current_step >= obstacle.vanish_frames.len() {
                            // Finished vanishing -> Hide
                            *visibility = Visibility::Hidden;
                            // Ensure collision is gone
                            commands.entity(entity).remove::<Collider>();
                        } else if let Some(atlas) = &mut sprite.texture_atlas {
                            atlas.index = obstacle.vanish_frames[obstacle.current_step];
                        }
                    } else {
                        // Loop until vanish
                        obstacle.current_step =
                            (obstacle.current_step + 1) % obstacle.loop_frames.len();
                        if let Some(atlas) = &mut sprite.texture_atlas {
                            atlas.index = obstacle.loop_frames[obstacle.current_step];
                        }
                    }
                }
            }
        }
    }
}
