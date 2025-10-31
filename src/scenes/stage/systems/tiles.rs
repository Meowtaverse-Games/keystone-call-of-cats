use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    plugins::tiled::*,
    plugins::{design_resolution::ScaledViewport, tiled::TileShape},
    scenes::stage::components::StageTile,
};

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

    let map_tile_dimensions = tiled_map_assets
        .tile_layers()
        .fold(UVec2::ZERO, |acc, layer| {
            let width = layer.width().max(0);
            let height = layer.height().max(0);
            UVec2::new(acc.x.max(width), acc.y.max(height))
        });

    info!("Map tile dimensions: {:?}", map_tile_dimensions);

    let raw_tile_size = tileset
        .image()
        .map(|image| image.tile_size)
        .unwrap_or(UVec2::new(32, 32));

    let base_tile_size = Vec2::new(raw_tile_size.x.max(1) as f32, raw_tile_size.y.max(1) as f32);

    let viewport_size = viewport.size;

    let map_pixel_size = Vec2::new(
        map_tile_dimensions.x as f32 * base_tile_size.x,
        map_tile_dimensions.y as f32 * base_tile_size.y,
    );

    let scale_x = viewport_size.x / map_pixel_size.x;
    let scale_y = viewport_size.y / map_pixel_size.y;
    let scale = scale_x.min(scale_y).max(f32::EPSILON);
    let tile_size = base_tile_size * scale;

    commands.entity(stage_root).with_children(|parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>| {
        for (layer_index, layer) in tiled_map_assets.tile_layers().enumerate() {
            let name = &layer.name;

            let layer_z = match name {
                n if n.starts_with("Background") => -10.0 + layer_index as f32 * 0.01,
                n if n.starts_with("Ground") => 0.0 + layer_index as f32 * 0.01,
                _ => layer_index as f32 * 0.01,
            };

            info!("Layer name: {}, z: {}", name, layer_z);

            for (x, y) in layer.tile_positions() {
                spawn_tile_entity(
                    parent,
                    &layer,
                    tileset,
                    x,
                    y,
                    tile_size,
                    base_tile_size,
                    viewport_size,
                    scale,
                    layer_z,
                );
            }
        }
    });
}

fn spawn_tile_entity(
    parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>,
    layer: &TileLayer,
    tileset: &Tileset,
    x: u32,
    y: u32,
    tile_size: Vec2,
    base_tile_size: Vec2,
    viewport_size: Vec2,
    scale: f32,
    layer_z: f32,
) {
    let Some(tile) = layer.tile(x, y) else {
        return;
    };
    let Some(tile_sprite) = tileset.atlas_sprite(tile.id) else {
        return;
    };

    let tile_x = (x as f32 + 0.5) * tile_size.x - viewport_size.x / 2.0;
    let tile_y = -((y as f32 + 0.5) * tile_size.y - viewport_size.y / 2.0);
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    let transform = Transform::from_xyz(tile_x, tile_y, layer_z)
        .with_scale(Vec3::new(scale, scale, 1.0));

    if tile.shapes.is_empty() {
        parent.spawn((StageTile, image, transform));
        return;
    }

    let colliders = tile
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
                    -base_tile_size.x / 2.0 + (*width + *x) / 2.0 + *x / 2.0,
                    base_tile_size.y / 2.0 - (*height + *y) / 2.0 - *y / 2.0,
                );
                let rot = Rotation::degrees(0.0);
                (pos, rot, collider)
            }
        })
        .collect::<Vec<_>>();

    parent.spawn((
        StageTile,
        image,
        transform,
        RigidBody::Static,
        Collider::compound(colliders),
    ));
}
