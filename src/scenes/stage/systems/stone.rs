use avian2d::prelude::*;
use bevy::prelude::*;

use crate::scenes::stage::components::StoneRune;

const STONE_ATLAS_PATH: &str = "images/spr_allrunes_spritesheet_xx.png";
const STONE_TILE_SIZE: UVec2 = UVec2::new(64, 64);
const STONE_SHEET_COLUMNS: u32 = 10;
const STONE_SHEET_ROWS: u32 = 7;
const STONE_TILE_COORD: UVec2 = UVec2::new(2, 4);
const STONE_SCALE: f32 = 1.6;

pub fn spawn_stone_display(
    commands: &mut Commands,
    stage_root: Entity,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) {
    let texture = asset_server.load(STONE_ATLAS_PATH);
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        STONE_TILE_SIZE,
        STONE_SHEET_COLUMNS,
        STONE_SHEET_ROWS,
        None,
        None,
    ));

    let tile_index = atlas_index(STONE_TILE_COORD);
    let atlas = TextureAtlas {
        layout: layout.clone(),
        index: tile_index,
    };

    commands.entity(stage_root).with_children(|parent| {
        parent.spawn((
            StoneRune,
            Sprite::from_atlas_image(texture, atlas),
            Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(STONE_SCALE)),
            RigidBody::Static,
            Collider::rectangle(
                (STONE_TILE_SIZE.x as f32) * 0.5,
                (STONE_TILE_SIZE.y as f32) * 0.5,
            ),
            DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
        ));
    });
}

fn atlas_index(coord: UVec2) -> usize {
    (coord.y as usize) * (STONE_SHEET_COLUMNS as usize) + coord.x as usize
}
