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

    let viewport_size = viewport.size;
    let tile_size = tileset.tile_size();
    let (real_tile_size, scale) =
        tiled_map_assets.scaled_tile_size_and_scale(viewport_size, tile_size);

    commands.entity(stage_root).with_children(
        |parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>| {
            for (layer_index, layer) in tiled_map_assets.tile_layers().enumerate() {
                for (x, y) in layer.tile_positions() {
                    spawn_tile_entity(
                        parent,
                        layer_index,
                        &layer,
                        tileset,
                        (x, y, real_tile_size, tile_size, viewport_size, scale),
                    );
                }
            }
        },
    );
}

fn image_from_tileset(tileset: &Tileset, id: u32) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}

fn spawn_tile_entity(
    parent: &mut bevy_ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>,
    layer_index: usize,
    layer: &TileLayer,
    tileset: &Tileset,
    (x, y, tile_size, base_tile_size, viewport_size, scale): (u32, u32, Vec2, Vec2, Vec2, f32),
) {
    let name = &layer.name;
    if name != "Background" {
        return;
    }

    let Some(tile) = layer.tile(x, y) else {
        return;
    };
    let Some(image) = image_from_tileset(tileset, tile.id) else {
        return;
    };

    let layer_z = match name {
        n if n.starts_with("Background") => -10.0 + layer_index as f32 * 0.01,
        n if n.starts_with("Ground") => 0.0 + layer_index as f32 * 0.01,
        _ => layer_index as f32 * 0.01,
    };

    let tile_x = (x as f32 + 0.5) * tile_size.x - viewport_size.x / 2.0;
    let tile_y = -((y as f32 + 0.5) * tile_size.y - viewport_size.y / 2.0);
    let transform =
        Transform::from_xyz(tile_x, tile_y, layer_z).with_scale(Vec3::new(scale, scale, 1.0));

    if x != 0 && y != 0 && x != 29 && y != 19 {
        parent.spawn((StageTile, image, transform));
        return;
    }

    // let colliders = tile
    //     .shapes
    //     .iter()
    //     .map(|shape| match shape {
    //         TileShape::Rect {
    //             width,
    //             height,
    //             x,
    //             y,
    //         } => {
    //             let collider = Collider::rectangle(*width, *height);
    //             let pos = Position::from_xy(
    //                 -base_tile_size.x / 2.0 + (*width + *x) / 2.0 + *x / 2.0,
    //                 base_tile_size.y / 2.0 - (*height + *y) / 2.0 - *y / 2.0,
    //             );
    //             let rot = Rotation::degrees(0.0);
    //             (pos, rot, collider)
    //         }
    //     })
    //     .collect::<Vec<_>>();

    let x = 0.0;
    let y = 0.0;
    let width = 16.0;
    let height = 16.0;

    let collider = Collider::rectangle(width, height);
    let pos = Position::from_xy(
        -base_tile_size.x / 2.0 + (width + x) / 2.0 + x / 2.0,
        base_tile_size.y / 2.0 - (height + y) / 2.0 - y / 2.0,
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
