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
    placed_chunks: &Map,
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
                if x < placed_chunks.boundary_margin.0 - 1
                    || x > map_size_x - placed_chunks.boundary_margin.0
                {
                    // skip wall columns
                    continue;
                }

                for y in 0..map_size_y {
                    if y < placed_chunks.boundary_margin.1 - 1
                        || y > map_size_y - placed_chunks.boundary_margin.1
                    {
                        // skip wall rows
                        continue;
                    }

                    let is_wall_x = x < placed_chunks.boundary_margin.0
                        || x >= map_size_x - placed_chunks.boundary_margin.0;
                    let is_wall_y = y < placed_chunks.boundary_margin.1
                        || y >= map_size_y - placed_chunks.boundary_margin.1;

                    let (is_edge_left, is_edge_right) = (
                        x == placed_chunks.boundary_margin.0 - 1,
                        x == map_size_x - placed_chunks.boundary_margin.0,
                    );
                    let (is_edge_top, is_edge_bottom) = (
                        y == placed_chunks.boundary_margin.1 - 1,
                        y == map_size_y - placed_chunks.boundary_margin.1,
                    );

                    let is_boundary = is_wall_x || is_wall_y;

                    let tile_x = (x as f32 + 0.5) * real_tile_size.x - viewport_size.x / 2.0;
                    let tile_y = -((y as f32 + 0.5) * real_tile_size.y - viewport_size.y / 2.0);

                    let mut transform = Transform::from_xyz(tile_x, tile_y, -20.0)
                        .with_scale(Vec3::new(scale, scale, 1.0));

                    let tile_id = if is_boundary
                        && (is_edge_left || is_edge_right || is_edge_top || is_edge_bottom)
                    {
                        if is_edge_left && is_edge_top {
                            112
                        } else if placed_chunks.boundary_margin.0 == x && is_edge_top {
                            270
                        } else if placed_chunks.boundary_margin.0 + 1 == x && is_edge_top {
                            transform.rotate_z((90.0f32).to_radians());
                            114
                        } else if is_edge_right && is_edge_top {
                            130
                        } else if is_edge_left && is_edge_bottom {
                            115
                        } else if (map_size_x - placed_chunks.boundary_margin.0 - 2) == x
                            && is_edge_bottom
                        {
                            transform.rotate_z((270.0f32).to_radians());
                            114
                        } else if (map_size_x - placed_chunks.boundary_margin.0 - 1) == x
                            && is_edge_bottom
                        {
                            270
                        } else if is_edge_right && is_edge_bottom {
                            132
                        } else if is_edge_top {
                            95
                        } else if is_edge_bottom {
                            133
                        } else if is_edge_left {
                            112
                        } else if is_edge_right {
                            132
                        } else {
                            61
                        }
                    } else {
                        background_tile_id(&mut rng)
                    };

                    let image = image_from_tileset(&tileset, tile_id as usize).unwrap();
                    spawn_boundary_tile(parent, image, transform, tile_size, is_boundary);

                    if x == placed_chunks.boundary_margin.0
                        && y <= placed_chunks.boundary_margin.1 + 2
                    {
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

                    if x == map_size_x - placed_chunks.boundary_margin.0 - 1
                        && y >= map_size_y - placed_chunks.boundary_margin.1 - 1
                    {
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

                    // Glass
                    if is_edge_bottom
                        && placed_chunks.tile_by_position((x, y - 1)).is_none()
                        && rng.random_bool(0.2)
                    {
                        let glass_image = image_from_tileset(&tileset, 226).unwrap();
                        let transform = Transform::from_xyz(
                            tile_x,
                            -(((y - 1) as f32 + 0.5) * real_tile_size.y - viewport_size.y / 2.0),
                            20.0,
                        )
                        .with_scale(Vec3::new(scale, scale, 1.0));
                        spawn_boundary_tile(parent, glass_image, transform, tile_size, false);
                    }
                }
            }

            for ((x, y), kind) in placed_chunks.map_iter() {
                let Some(tile_id) = tile_id_for_kind(&mut rng, kind) else {
                    continue;
                };
                let Some(image) = image_from_tileset(&tileset, tile_id as usize) else {
                    continue;
                };

                let tile_x = (x as f32 + 0.5) * real_tile_size.x - viewport_size.x / 2.0;
                let tile_y = (y as f32 + 0.5) * real_tile_size.y - viewport_size.y / 2.0;
                let transform = Transform::from_xyz(tile_x, tile_y, -5.0)
                    .with_scale(Vec3::new(scale, scale, 1.0));

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
                    Collider::compound(shapes.clone()),
                ));

                if kind == TileKind::Solid && rng.random_bool(0.3) {
                    let Some(moss_image) = image_from_tileset(&tileset, rng.random_range(224..226))
                    else {
                        continue;
                    };
                    let transform = transform.with_translation(Vec3::new(tile_x, tile_y, -2.5));
                    parent.spawn((
                        StageTile,
                        moss_image,
                        transform,
                        RigidBody::Static,
                        Collider::compound(shapes),
                    ));
                }
            }
        },
    );
}

fn image_from_tileset(tileset: &Tileset, id: usize) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id as u32)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}

fn tile_id_for_kind(rng: &mut impl Rng, kind: TileKind) -> Option<u32> {
    match kind {
        TileKind::Solid => Some(rng.random_range(235..237)),
        TileKind::Goal => None, // Some(178),
        TileKind::Wall => None, // Some(152),
        TileKind::PlayerSpawn | TileKind::Stone | TileKind::Obstacle => None,
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

pub fn restore_dug_tiles(
    mut commands: Commands,
    mut query: Query<(Entity, &crate::scenes::stage::components::DugTile)>,
    editor_state: Res<crate::scenes::stage::systems::ui::ScriptEditorState>,
) {
    if !editor_state.pending_player_reset {
        return;
    }

    for (entity, dug_tile) in &mut query {
        commands
            .entity(entity)
            .remove::<crate::scenes::stage::components::DugTile>()
            .remove::<Visibility>() // Remove hidden
            .insert(Visibility::Inherited) // Restore visibility
            .insert(dug_tile.collider.clone()); // Restore physics
    }
}

pub fn despawn_placed_tiles(
    mut commands: Commands,
    query: Query<Entity, With<crate::scenes::stage::components::PlacedTile>>,
    editor_state: Res<crate::scenes::stage::systems::ui::ScriptEditorState>,
) {
    if !editor_state.pending_player_reset {
        return;
    }

    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
