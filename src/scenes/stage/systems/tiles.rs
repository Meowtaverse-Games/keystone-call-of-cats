use avian2d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

use crate::{
    core::domain::chunk_grammar_map::{self, MAP_SIZE, TileKind},
    plugins::{design_resolution::ScaledViewport, tiled::*},
    scenes::stage::components::StageTile,
};

const CHUNK_GRAMMAR_CONFIG_PATH: &str = "assets/chunk_grammar_map/tutorial.ron";

pub fn spawn_tiles(
    commands: &mut Commands,
    stage_root: Entity,
    tiled_map_assets: &TiledMapAssets,
    viewport: &ScaledViewport,
) {
    let Some(tileset) = tiled_map_assets.tilesets().first() else {
        warn!("Stage setup: no tilesets available");
        return;
    };

    let viewport_size = viewport.size;
    let tile_size = tileset.tile_size();
    let map_pixel_size = Vec2::new(
        MAP_SIZE.0 as f32 * tile_size.x,
        MAP_SIZE.1 as f32 * tile_size.y,
    );
    let scale = (viewport_size / map_pixel_size).min_element();
    let real_tile_size = tile_size * scale;

    let mut rng = rand::rng();
    let placed_chunks = match chunk_grammar_map::generate_random_layout_from_file(
        &mut rng,
        CHUNK_GRAMMAR_CONFIG_PATH,
    ) {
        Ok(chunks) => chunks,
        Err(err) => {
            warn!(
                "Stage setup: failed to generate tiles from chunk grammar config '{}': {err}",
                CHUNK_GRAMMAR_CONFIG_PATH
            );
            return;
        }
    };

    chunk_grammar_map::print_ascii_map(&chunk_grammar_map::build_tile_char_map(&placed_chunks));

    let mut tiles: Vec<_> = chunk_grammar_map::build_tile_kind_map(&placed_chunks)
        .into_iter()
        .collect();
    tiles.sort_by_key(|((x, y), _)| (*y, *x));

    commands.entity(stage_root).with_children(
        |parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>| {
            for x in 0..MAP_SIZE.0 {
                for y in 0..MAP_SIZE.1 {
                    let tile_x = (x as f32 + 0.5) * real_tile_size.x - viewport_size.x / 2.0;
                    let tile_y = -((y as f32 + 0.5) * real_tile_size.y - viewport_size.y / 2.0);

                    let transform = Transform::from_xyz(tile_x, tile_y, -10.0)
                        .with_scale(Vec3::new(scale, scale, 1.0));
                    let is_boundary =
                        x == 0 || y == 0 || x == MAP_SIZE.0 - 1 || y == MAP_SIZE.1 - 1;
                    let background_ids = [
                        251, 252, 253, 254, 268, 269, 270, 271, 285, 286, 287, 288, 302, 303, 304,
                        305,
                    ];
                    let tile_id = if is_boundary {
                        if x == 0 && y == 0 {
                            113
                        } else if x == MAP_SIZE.0 - 1 && y == 0 {
                            130
                        } else if x == 0 && y == MAP_SIZE.1 - 1 {
                            115
                        } else if x == MAP_SIZE.0 - 1 && y == MAP_SIZE.1 - 1 {
                            168
                        } else if y == 0 {
                            95
                        } else if y == MAP_SIZE.1 - 1 {
                            133
                        } else if x == 0 {
                            112
                        } else if x == MAP_SIZE.0 - 1 {
                            132
                        } else {
                            134 // Fallback, should not happen
                        }
                    } else {
                        let index = rng.random_range(0..(background_ids.len()));
                        background_ids[index]
                    };

                    let Some(image) = image_from_tileset(tileset, tile_id as usize) else {
                        continue;
                    };
                    spawn_boundary_tile(parent, image, transform, tile_size, is_boundary);
                }
            }

            for ((x, y), kind) in tiles {
                let Some(tile_id) = tile_id_for_kind(kind) else {
                    continue;
                };
                let Some(image) = image_from_tileset(tileset, tile_id as usize) else {
                    continue;
                };

                let tile_x = (x as f32 + 1.5) * real_tile_size.x - viewport_size.x / 2.0;
                let tile_y = ((y as f32 + 4.0) * real_tile_size.y - viewport_size.y / 2.0);
                let transform = Transform::from_xyz(tile_x, tile_y, -10.0)
                    .with_scale(Vec3::new(scale, scale, 1.0));

                parent.spawn((StageTile, image, transform));
            }
        },
    );
}

fn image_from_tileset(tileset: &Tileset, id: usize) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id as u32)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}

fn tile_id_for_kind(kind: TileKind) -> Option<u32> {
    info!("Getting tile ID for kind: {:?}", kind);
    match kind {
        TileKind::Solid => Some(235),
        TileKind::PlayerSpawn => Some(408),
        TileKind::Goal => Some(194),
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
