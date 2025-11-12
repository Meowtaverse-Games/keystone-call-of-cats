use bevy::{asset::Assets, prelude::*};
use std::sync::Arc;

use crate::resources::tiled::{TiledLoaderConfig, TiledMapAssets, TiledTilesetImage, Tileset};

pub fn load_tiled_assets(
    mut commands: Commands,
    config: Res<TiledLoaderConfig>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut loader = tiled::Loader::new();

    let tsx = match loader.load_tsx_tileset(&config.tsx_path) {
        Ok(tileset) => tileset,
        Err(err) => {
            error!(
                target: "tiled",
                "Failed to load TSX tilesets from '{}': {err}",
                config.tsx_path
            );
            return;
        }
    };

    let tileset = load_tileset(&asset_server, &mut layouts, &tsx);
    commands.insert_resource(TiledMapAssets {
        tsx: Arc::new(tsx),
        tileset,
    });
}

fn load_tileset(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    tileset: &tiled::Tileset,
) -> Tileset {
    let image = tileset
        .image
        .as_ref()
        .map(|image| create_tileset_image(tileset, image, asset_server, layouts));

    Tileset { image }
}

fn create_tileset_image(
    tileset: &tiled::Tileset,
    image: &tiled::Image,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> TiledTilesetImage {
    let path = normalize_asset_path(&image.source);
    info!(target: "tiled", "Loading tileset image from path: {}", path);
    let texture = asset_server.load(path.clone());

    let columns = tileset.columns.max(1);
    let tile_count = tileset.tilecount;
    let rows = tile_count.div_ceil(columns).max(1);

    let mut layout = TextureAtlasLayout::from_grid(
        UVec2::new(tileset.tile_width, tileset.tile_height),
        columns,
        rows,
        Some(UVec2::new(tileset.spacing, tileset.spacing)),
        Some(UVec2::new(tileset.margin, tileset.margin)),
    );
    info!(
        target: "tiled",
        "Tileset '{}' layout: {} columns x {} rows (tile size: {}x{}, spacing: {}, margin: {})",
        tileset.name,
        columns,
        rows,
        tileset.tile_width,
        tileset.tile_height,
        tileset.spacing,
        tileset.margin
    );

    layout.textures.truncate(tile_count as usize);

    let layout = layouts.add(layout);

    TiledTilesetImage {
        texture,
        layout,
        tile_size: UVec2::new(tileset.tile_width, tileset.tile_height),
    }
}

fn normalize_asset_path(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy().replace('\\', "/");

    if let Some(stripped) = path_str.strip_prefix("assets/") {
        stripped.to_string()
    } else {
        path_str
    }
}
