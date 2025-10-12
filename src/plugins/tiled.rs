use std::sync::Arc;

use bevy::asset::Assets;
use bevy::math::UVec2;
use bevy::prelude::*;
use tiled::{Loader, Map, Tileset};

/// Configures how the [`TiledPlugin`] loads Tiled data.
#[derive(Resource, Clone)]
pub struct TiledLoaderConfig {
    pub map_path: String,
}

impl Default for TiledLoaderConfig {
    fn default() -> Self {
        Self {
            map_path: "assets/tiled/stage1-1.tmx".to_string(),
        }
    }
}

pub struct TiledPlugin {
    config: TiledLoaderConfig,
}

impl Default for TiledPlugin {
    fn default() -> Self {
        Self {
            config: TiledLoaderConfig::default(),
        }
    }
}

impl TiledPlugin {
    pub fn new(map_path: impl Into<String>) -> Self {
        Self {
            config: TiledLoaderConfig {
                map_path: map_path.into(),
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
    map: Arc<Map>,
    tilesets: Vec<TiledTileset>,
}

impl TiledMapAssets {
    pub fn map(&self) -> Arc<Map> {
        Arc::clone(&self.map)
    }

    pub fn tileset(&self, index: usize) -> Option<&TiledTileset> {
        self.tilesets.get(index)
    }
}

#[derive(Clone)]
pub struct TiledTileset {
    tileset: Arc<Tileset>,
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
    let mut loader = Loader::new();

    let map = match loader.load_tmx_map(&config.map_path) {
        Ok(map) => map,
        Err(err) => {
            error!(target: "tiled", "Failed to load TMX map '{}': {err}", config.map_path);
            return;
        }
    };

    let tilesets = map
        .tilesets()
        .iter()
        .map(|tileset| load_tileset(tileset, &asset_server, &mut layouts))
        .collect::<Vec<_>>();

    commands.insert_resource(TiledMapAssets {
        map: Arc::new(map),
        tilesets,
    });
}

fn load_tileset(
    tileset: &Arc<Tileset>,
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
    tileset: &Tileset,
    image: &tiled::Image,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> TiledTilesetImage {
    let path = normalize_asset_path(&image.source);
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
