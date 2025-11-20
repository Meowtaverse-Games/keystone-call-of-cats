use avian2d::prelude::*;
use bevy::prelude::*;

use rand::Rng;

use crate::{
    resources::{chunk_grammar_map::*, design_resolution::ScaledViewport, tiled::*},
    scenes::stage::components::StageTile,
};

const BACKGROUND_IDS: [u32; 16] = [
    251, 252, 253, 254, 268, 269, 270, 271, 285, 286, 287, 288, 302, 303, 304, 305,
];

fn background_tile_id(rng: &mut rand::rngs::ThreadRng) -> u32 {
    let index = rng.random_range(0..(BACKGROUND_IDS.len()));
    BACKGROUND_IDS[index]
}

pub fn spawn_tiles(
    commands: &mut Commands,
    stage_root: Entity,
    tiled_map_assets: &TiledMapAssets,
    placed_chunks: &PlacedChunkLayout,
    viewport: &ScaledViewport,
) {
    let tileset = tiled_map_assets.tileset.clone();

    let mut rng = rand::rng();

    let (map_size_x, map_size_y) = placed_chunks.map_size;

    let viewport_size = viewport.size;
    let tile_size = tileset.tile_size();
    let map_pixel_size = Vec2::new(
        map_size_x as f32 * tile_size.x,
        map_size_y as f32 * tile_size.y,
    );
    let scale = (viewport_size / map_pixel_size).min_element();
    let real_tile_size = tile_size * scale;

    commands.entity(stage_root).with_children(
        |parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>| {
            for x in 0..map_size_x {
                for y in 0..map_size_y {
                    let tile_x = (x as f32 + 0.5) * real_tile_size.x - viewport_size.x / 2.0;
                    let tile_y = -((y as f32 + 0.5) * real_tile_size.y - viewport_size.y / 2.0);

                    let mut transform = Transform::from_xyz(tile_x, tile_y, -20.0)
                        .with_scale(Vec3::new(scale, scale, 1.0));
                    let is_boundary =
                        x == 0 || y == 0 || x == map_size_x - 1 || y == map_size_y - 1;

                    let tile_id = if is_boundary {
                        if x == 0 && y == 0 {
                            112
                        } else if x == 1 && y == 0 {
                            270
                        } else if x == 2 && y == 0 {
                            transform.rotate_z((90.0f32).to_radians());
                            114
                        } else if x == map_size_x - 1 && y == 0 {
                            130
                        } else if x == 0 && y == map_size_y - 1 {
                            115
                        } else if x == map_size_x - 1 && y == map_size_y - 1 {
                            168
                        } else if y == 0 {
                            95
                        } else if y == map_size_y - 1 {
                            133
                        } else if x == 0 {
                            112
                        } else if x == map_size_x - 1 {
                            132
                        } else {
                            0 // Fallback, should not happen
                        }
                    } else {
                        background_tile_id(&mut rng)
                    };

                    let image = image_from_tileset(&tileset, tile_id as usize).unwrap();
                    spawn_boundary_tile(parent, image, transform, tile_size, is_boundary);

                    if x == 1 && y <= 2 {
                        let ladder_image = image_from_tileset(&tileset, 178).unwrap();
                        spawn_boundary_tile(
                            parent,
                            ladder_image,
                            Transform::from_xyz(tile_x, tile_y, -8.0)
                                .with_scale(Vec3::new(scale, scale, 1.0)),
                            tile_size,
                            false,
                        );
                    }
                }
            }

            for ((x, y), kind) in placed_chunks.map_iter() {
                let Some(tile_id) = tile_id_for_kind(placed_chunks, kind) else {
                    continue;
                };
                let Some(image) = image_from_tileset(&tileset, tile_id as usize) else {
                    continue;
                };

                let tile_x = (x as f32 + 0.5) * real_tile_size.x - viewport_size.x / 2.0;
                let tile_y = (y as f32 + 0.5) * real_tile_size.y - viewport_size.y / 2.0;
                let transform = Transform::from_xyz(tile_x, tile_y, -5.0)
                    .with_scale(Vec3::new(scale, scale, 1.0));

                // spawn_boundary_tile(parent, image, transform, tile_size, true);

                let Some(tile) = tiled_map_assets.tile(tile_id) else {
                    continue;
                };

                let shapes = tile
                    .shapes
                    .iter()
                    .map(|shape| match shape {
                        TileShape::Rect {
                            width,
                            height,
                            x,
                            y,
                        } => {
                            let collider = Collider::rectangle(*width, *height);
                            let pos = Position::from_xy(
                                -tile_size.x / 2.0 + (width + x) / 2.0 + x / 2.0,
                                tile_size.y / 2.0 - (height + y) / 2.0 - y / 2.0,
                            );
                            let rot = Rotation::degrees(0.0);
                            (pos, rot, collider)
                        }
                    })
                    .collect::<Vec<_>>();

                if shapes.is_empty() {
                    parent.spawn((StageTile, image, transform));
                    continue;
                }
                parent.spawn((
                    StageTile,
                    image,
                    transform,
                    RigidBody::Static,
                    Collider::compound(shapes),
                ));
            }
        },
    );
}

fn image_from_tileset(tileset: &Tileset, id: usize) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id as u32)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}

fn tile_id_for_kind(_placed_chunks: &PlacedChunkLayout, kind: TileKind) -> Option<u32> {
    match kind {
        TileKind::Solid => Some(235),
        TileKind::Goal => Some(194),
        TileKind::Wall => Some(152),
        TileKind::PlayerSpawn | TileKind::Stone => None,
    }
}

fn spawn_boundary_tile(
    parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>,
    image: Sprite,
    transform: Transform,
    base_tile_size: Vec2,
    is_boundary: bool,
) {
    let width = base_tile_size.x;
    let height = base_tile_size.y;

    if !is_boundary {
        parent.spawn((StageTile, image, transform));
        return;
    }

    let collider = Collider::rectangle(width, height);
    let pos = Position::from_xy(
        -base_tile_size.x / 2.0 + width / 2.0,
        base_tile_size.y / 2.0 - height / 2.0,
    );
    let rot = Rotation::degrees(0.0);

    parent.spawn((
        StageTile,
        image,
        transform,
        RigidBody::Static,
        Collider::compound(Vec::from([(pos, rot, collider)])),
    ));
}
