use std::sync::Arc;

use bevy::asset::Assets;
use bevy::math::UVec2;
use bevy::prelude::*;

use tiled::{self as tiled_rs};

mod object_layer;
mod tile_layer;

pub use object_layer::{Object, ObjectLayer};
pub use tile_layer::{Tile, TileLayer, TileShape};

/// Configures how the [`TiledPlugin`] loads Tiled data.
#[derive(Resource, Clone)]
pub struct TiledLoaderConfig {
    pub map_path: String,
    pub tsx_path: String,
}

// A plugin that loads Tiled maps and tilesets at startup.
pub struct TiledPlugin {
    config: TiledLoaderConfig,
}

impl TiledPlugin {
    pub fn new(map_path: impl Into<String>, tsx_path: impl Into<String>) -> Self {
        Self {
            config: TiledLoaderConfig {
                map_path: map_path.into(),
                tsx_path: tsx_path.into(),
            },
        }
    }
}

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .add_systems(Startup, load_tiled_assets);
    }
}

#[derive(Resource)]
pub struct TiledMapAssets {
    _tsx: Arc<tiled_rs::Tileset>,
    map: Arc<tiled_rs::Map>,
    tilesets: Vec<Tileset>,
}

impl TiledMapAssets {
    pub fn tilesets(&self) -> &[Tileset] {
        &self.tilesets
    }

    pub fn tile_layers<'a>(&'a self) -> impl Iterator<Item = TileLayer<'a>> + 'a {
        self.map.layers().filter_map(|layer| {
            let tiled_rs::LayerType::Tiles(tile_layer) = layer.layer_type() else {
                return None;
            };
            Some(TileLayer::new(layer.name.clone(), tile_layer))
        })
    }

    pub fn object_layers<'a>(&'a self) -> impl Iterator<Item = ObjectLayer<'a>> + 'a {
        self.map.layers().filter_map(|layer| {
            let tiled_rs::LayerType::Objects(object_layer) = layer.layer_type() else {
                return None;
            };
            Some(ObjectLayer::new(layer.name.clone(), object_layer))
        })
    }
}

#[derive(Clone)]
pub struct Tileset {
    tileset: Arc<tiled_rs::Tileset>,
    image: Option<TiledTilesetImage>,
}

impl Tileset {
    pub fn image(&self) -> Option<&TiledTilesetImage> {
        self.image.as_ref()
    }

    pub fn atlas_sprite(&self, local_id: u32) -> Option<TiledAtlasSprite> {
        let image = self.image.as_ref()?;
        if local_id >= self.tileset.tilecount {
            return None;
        }

        Some(TiledAtlasSprite {
            texture: image.texture.clone(),
            atlas: TextureAtlas {
                layout: image.layout.clone(),
                index: local_id as usize,
            },
        })
    }
}

#[derive(Clone)]
pub struct TiledTilesetImage {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub tile_size: UVec2,
}

#[derive(Clone)]
pub struct TiledAtlasSprite {
    pub texture: Handle<Image>,
    pub atlas: TextureAtlas,
}

fn load_tiled_assets(
    mut commands: Commands,
    config: Res<TiledLoaderConfig>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut loader = tiled_rs::Loader::new();

    let map = match loader.load_tmx_map(&config.map_path) {
        Ok(map) => map,
        Err(err) => {
            error!(target: "tiled", "Failed to load TMX map '{}': {err}", config.map_path);
            return;
        }
    };

    let tsx = match loader.load_tsx_tileset(&config.tsx_path) {
        Ok(tileset) => tileset,
        Err(err) => {
            error!(target: "tiled", "Failed to load TSX tilesets from '{}': {err}", config.tsx_path);
            return;
        }
    };

    let tilesets = map
        .tilesets()
        .iter()
        .map(|tileset| load_tileset(tileset, &asset_server, &mut layouts))
        .collect::<Vec<_>>();

    map.tilesets().iter().for_each(|tileset| {
        info!(target: "tiled", "Loaded tileset: {}", tileset.name);
    });

    commands.insert_resource(TiledMapAssets {
        _tsx: Arc::new(tsx),
        map: Arc::new(map),
        tilesets,
    });
}

fn load_tileset(
    tileset: &Arc<tiled_rs::Tileset>,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Tileset {
    let image = tileset
        .image
        .as_ref()
        .map(|image| create_tileset_image(tileset, image, asset_server, layouts));

    Tileset {
        tileset: Arc::clone(tileset),
        image,
    }
}

fn create_tileset_image(
    tileset: &tiled_rs::Tileset,
    image: &tiled_rs::Image,
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
    info!(target: "tiled", "Tileset '{}' layout: {} columns x {} rows (tile size: {}x{}, spacing: {}, margin: {})", tileset.name, columns, rows, tileset.tile_width, tileset.tile_height, tileset.spacing, tileset.margin);

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
