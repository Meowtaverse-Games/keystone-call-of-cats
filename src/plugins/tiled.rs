use std::sync::Arc;

use bevy::asset::Assets;
use bevy::math::UVec2;
use bevy::prelude::*;

use tiled as tiled_rs;

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
    tsx: Arc<tiled_rs::Tileset>,
    map: Arc<tiled_rs::Map>,
    tilesets: Vec<TiledTileset>,
}

impl TiledMapAssets {
    pub fn tilesets(&self) -> &[TiledTileset] {
        &self.tilesets
    }

    pub fn layers(&self) -> impl Iterator<Item = Layer> + '_ {
        self.map.layers().map(|layer| {
            let tag = match layer.layer_type() {
                tiled_rs::LayerType::Tiles(_) => LayerTag::Tile,
                tiled_rs::LayerType::Objects(_) => LayerTag::Object,
                tiled_rs::LayerType::Image(_) => LayerTag::Image,
                tiled_rs::LayerType::Group(_) => LayerTag::Group,
            };

            Layer {
                name: layer.name.clone(),
                tag,
                tile_layer: layer.as_tile_layer().unwrap(),
            }
        })
    }

    pub fn tile(x: i32, y: i32) -> Option<Tile> {
        let tile_layer = layer.as_tile_layer().unwrap();
        info!(
            "Layer '{}' has dimensions {}x{}",
            layer.name,
            tile_layer.width().unwrap(),
            tile_layer.height().unwrap()
        );
        info!("Tiles in layer '{:?}", tile_layer);
        for x in 0..tile_layer.width().unwrap() {
            for y in 0..tile_layer.height().unwrap() {
                if let Some(tile) = tile_layer.get_tile(x as i32, y as i32) {
                    if let Some(tile_data) = tile.get_tile() {
                        // info!("Tile at ({}, {}): ID {}: {:?}", x, y, tile.id(), tile_data.properties);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum LayerTag {
    Tile,
    Object,
    Image,
    Group,
}

pub struct Layer<'map> {
    pub name: String,
    pub tag: LayerTag,
    tile_layer: tiled_rs::TileLayer<'map>,
}

#[derive(Clone)]
pub struct TiledTileset {
    tileset: Arc<tiled_rs::Tileset>,
    image: Option<TiledTilesetImage>,
}

impl TiledTileset {
    pub fn name(&self) -> &str {
        &self.tileset.name
    }

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
    pub path: String,
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
        Ok(tilesets) => tilesets,
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
        tsx: Arc::new(tsx),
        map: Arc::new(map),
        tilesets,
    });
}

fn load_tileset(
    tileset: &Arc<tiled_rs::Tileset>,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> TiledTileset {
    let image = tileset
        .image
        .as_ref()
        .map(|image| create_tileset_image(tileset, image, asset_server, layouts));

    TiledTileset {
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
    let rows = ((tile_count + columns - 1) / columns).max(1);

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
        path,
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
