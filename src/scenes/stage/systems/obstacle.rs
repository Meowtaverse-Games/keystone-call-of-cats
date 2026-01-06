use bevy::prelude::*;

#[derive(Component)]
pub struct AnimatedObstacle {
    pub timer: Timer,
    pub frame_indices: Vec<usize>,
    pub current_step: usize,
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

    let frame_indices = vec![195, 196, 197, 198, 212, 213, 214, 215];

    let obstacle_entity = commands
        .spawn((
            Sprite {
                image: texture,
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout,
                    index: frame_indices[0],
                }),
                ..default()
            },
            Transform::from_xyz(x, y, 10.0).with_scale(Vec3::splat(scale)),
            AnimatedObstacle {
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
                frame_indices,
                current_step: 0,
            },
        ))
        .id();
    commands.entity(stage_root).add_child(obstacle_entity);
}

pub fn animate_obstacle(time: Res<Time>, mut query: Query<(&mut AnimatedObstacle, &mut Sprite)>) {
    for (mut star, mut sprite) in &mut query {
        star.timer.tick(time.delta());
        if star.timer.just_finished() {
            star.current_step = (star.current_step + 1) % star.frame_indices.len();
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = star.frame_indices[star.current_step];
            }
        }
    }
}
