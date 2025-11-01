use avian2d::prelude::*;
use bevy::prelude::*;

pub const GOAL_OBJECT_ID: u32 = 194;

use crate::{plugins::design_resolution::ScaledViewport, plugins::tiled::*};

pub fn spawn_goal(
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
    info!(
        "Computed tile size!!!1: {:?}, scale: {}",
        real_tile_size, scale
    );

    commands.entity(stage_root).with_children(|parent| {
        let object_layer = tiled_map_assets.object_layer();
        let Some(goal_object) = object_layer.object_by_id(GOAL_OBJECT_ID) else {
            warn!("Stage setup: no goal object found in object layer");
            return;
        };
        info!("goal object: {:?}", goal_object);

        let object_x =
            goal_object.position.x * scale + real_tile_size.x / 2.0 - viewport_size.x / 2.0;
        let object_y =
            -((goal_object.position.y * scale - real_tile_size.y / 2.0) - viewport_size.y / 2.0);
        let transform =
            Transform::from_xyz(object_x, object_y, 0.0).with_scale(Vec3::new(scale, scale, 1.0));

        parent.spawn((
            image_from_tileset(tileset, goal_object.id).unwrap(),
            transform,
            RigidBody::Static,
            // Collider::compound(colliders),
        ));
    });
}

fn image_from_tileset(tileset: &Tileset, id: u32) -> Option<Sprite> {
    let tile_sprite = tileset.atlas_sprite(id)?;
    let image = Sprite::from_atlas_image(tile_sprite.texture, tile_sprite.atlas);
    Some(image)
}
