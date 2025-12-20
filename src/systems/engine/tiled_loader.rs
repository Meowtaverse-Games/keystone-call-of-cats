use bevy::{asset::Assets, prelude::*};

use crate::resources::tiled::{TiledLoaderConfig, TiledMapAssets, TiledTilesetImage, Tileset};

pub fn load_tiled_assets(
    mut commands: Commands,
    config: Res<TiledLoaderConfig>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let tileset = load_tileset(&asset_server, &mut layouts, &config.image_path);
    commands.insert_resource(TiledMapAssets { tileset });
}

fn load_tileset(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    image_path: &str,
) -> Tileset {
    let image = create_tileset_image(image_path, asset_server, layouts);
    Tileset { image: Some(image) }
}

fn create_tileset_image(
    path: &str,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> TiledTilesetImage {
    info!(target: "tiled", "Loading tileset image from path: {}", path);
    // Bevy's asset server handles relative paths from assets/ folder
    let texture = asset_server.load(path.to_string());

    // Hardcoded values from super-platfomer-assets.tsx
    let tile_width = 16;
    let tile_height = 16;
    let spacing = 3;
    let margin = 0;
    let columns = 17;
    let tile_count = 561;

    let rows = tile_count / columns + if tile_count % columns > 0 { 1 } else { 0 };

    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(tile_width, tile_height),
        columns,
        rows,
        Some(UVec2::new(spacing, spacing)),
        Some(UVec2::new(margin, margin)),
    );
    info!(
        target: "tiled",
        "Tileset 'super-platfomer-assets' (Manual) layout: {} columns x {} rows (tile size: {}x{}, spacing: {}, margin: {})",
        columns,
        rows,
        tile_width,
        tile_height,
        spacing,
        margin
    );

    let layout = layouts.add(layout);

    TiledTilesetImage {
        texture,
        layout,
        tile_size: UVec2::new(tile_width, tile_height),
    }
}
