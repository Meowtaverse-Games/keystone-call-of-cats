use std::sync::Arc;

use bevy::asset::Assets;
use bevy::math::UVec2;
use bevy::prelude::*;

use tiled::{self as tiled_rs};

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

    pub fn layers<'a>(&'a self) -> impl Iterator<Item = Box<dyn LayerTrait + 'a>> + 'a {
        self.map.layers().map(|layer| {
            let name = layer.name.clone();
            match layer.layer_type() {
                tiled_rs::LayerType::Tiles(_tile_layer) => Box::new(TileLayer::new(
                    name,
                    layer.as_tile_layer().unwrap(),
                )) as Box<dyn LayerTrait + 'a>,
                tiled_rs::LayerType::Objects(_object_layer) => Box::new(ObjectLayer::new(
                    name,
                    layer.as_object_layer().unwrap(),
                )) as Box<dyn LayerTrait + 'a>,
                _ => {
                    unimplemented!()
                }
            }
        })
    }

    // pub fn tile(&self, id: u32) -> Option<Tile> {
    //     let tileset = self.map.tilesets().first().unwrap();
    //     let Some(tile) = tileset.get_tile(id) else {
    //         info!("Tile ID {} not found in tileset {}.", id, tileset.name);
    //         return None;
    //     };

    //     // info!("Tile ID {} found in tileset {:?}.", id, *tile);

    //     if let Some(collision) = &tile.collision {
    //         info!("Tile ID {} has collision data.", id);
    //         let object_data = collision.object_data();

    //         Some(Tile {
    //             id,
    //             collision: tile.properties.get("collision").and_then(|v| {
    //                 if let tiled_rs::PropertyValue::BoolValue(b) = v {
    //                     Some(*b)
    //                 } else {
    //                     None
    //                 }
    //             }),
    //             shapes: object_data
    //                 .iter()
    //                 .map(|data| match data.shape {
    //                     tiled_rs::ObjectShape::Rect { width, height } => TileShape::Rect {
    //                         width,
    //                         height,
    //                         x: data.x,
    //                         y: data.y,
    //                     },
    //                     _ => {
    //                         unimplemented!()
    //                     }
    //                 })
    //                 .collect(),
    //         })
    //     } else {
    //         info!("Tile ID {} has no collision data.", id);
    //         None
    //     }

    //     /*
    //         Some(Tile {
    //             id: tile.id(),
    //             shapes: tile.shapes().map(|shape|
    //                 match shape {
    //                     tiled_rs::ObjectShape::Rect { width, height } => {
    //                         TileShape::Rect { width, height, x, y }
    //                     }
    //                     _ => {
    //                         // Handle other shapes as needed
    //                         unimplemented!()
    //                     }
    //                 })
    //             })
    //     } */
    // }
}

#[derive(Debug)]
pub enum TileShape {
    Rect {
        width: f32,
        height: f32,
        x: f32,
        y: f32,
    },
}

#[derive(Debug)]
pub enum LayerType {
    Tile,
    Object,
}

pub enum TileIndex {
    TilePosition (i32, i32),
    ObjectIndex (usize),
}

#[derive(Debug)]
pub struct Tile {
    pub id: u32,
    #[allow(dead_code)]
    pub collision: Option<bool>,
    pub shapes: Vec<TileShape>,
}

pub struct Layer {
    pub name: String,
}

pub trait LayerTrait {
    fn name(&self) -> &str;
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn tile_indexes(&self) -> Box<dyn Iterator<Item = TileIndex> + '_>;
    fn tile(&self, tile_index: TileIndex) -> Option<Tile>;
}

pub struct TileLayer<'map> {
    pub layer: Layer,
    pub inner_layer: tiled_rs::TileLayer<'map>,
}

impl<'map> TileLayer<'map> {
    fn new(name: String, tile_layer: tiled_rs::TileLayer<'map>) -> Self {
        Self {
            layer: Layer {
                name,
            },
            inner_layer: tile_layer,
        }
    }
}

impl<'map> LayerTrait for TileLayer<'map> {
    fn name(&self) -> &str {
        &self.layer.name
    }

    fn width(&self) -> i32 {
        self.inner_layer.width().map(|w| w as i32).unwrap_or(0)
    }

    fn height(&self) -> i32 {
        self.inner_layer.height().map(|h| h as i32).unwrap_or(0)
    }

    fn tile_indexes(&self) -> Box<dyn Iterator<Item = TileIndex> + '_> {
        let mut indexes = vec![];
        for x in 0..self.inner_layer.width().unwrap() as i32 {
            for y in 0..self.inner_layer.height().unwrap() as i32 {
                if let Some(_tile) = self.inner_layer.get_tile(x, y) {
                    indexes.push(TileIndex::TilePosition(x, y));
                }
            }
        }
        Box::new(indexes.into_iter())
    }

    fn tile(&self, tile_index: TileIndex) -> Option<Tile> {
        let tile = match tile_index {
            TileIndex::TilePosition(x, y) => self.inner_layer.get_tile(x, y)?,
            _ => return None,
        };

        tile.get_tile().map(|tile_data| {
            let Some(collision) = tile_data.collision.as_ref() else {
                return Tile {
                    id: tile.id(),
                    collision: None,
                    shapes: vec![],
                };
            };

            let object_data = collision.object_data();
            let shapes = object_data
                .iter()
                .map(|data| match data.shape {
                    tiled_rs::ObjectShape::Rect { width, height } => TileShape::Rect {
                        width,
                        height,
                        x: data.x,
                        y: data.y,
                    },
                    _ => {
                        unimplemented!()
                    }
                })
                .collect();

            Tile {
                id: tile.id(),
                // Retrieving a custom property named "collision" only if it exists and is a boolean
                collision: tile_data.properties.get("collision").and_then(|v| {
                    if let tiled_rs::PropertyValue::BoolValue(b) = v {
                        Some(*b)
                    } else {
                        None
                    }
                }),
                shapes,
            }
        })
    }
}

pub struct ObjectLayer<'map> {
    pub layer: Layer,
    pub inner_layer: tiled_rs::ObjectLayer<'map>,
}

impl<'map> ObjectLayer<'map> {
    fn new(name: String, object_layer: tiled_rs::ObjectLayer<'map>) -> Self {
        Self {
            layer: Layer {
                name,
            },
            inner_layer: object_layer,
        }
    }
}

impl<'map> LayerTrait for ObjectLayer<'map> {
    fn name(&self) -> &str {
        &self.layer.name
    }

    fn width(&self) -> i32 {
        self.inner_layer.map().width as i32
    }

    fn height(&self) -> i32 {
        self.inner_layer.map().height as i32
    }

    fn tile_indexes(&self) -> Box<dyn Iterator<Item = TileIndex> + '_> {
        let mut indexes = vec![];
        for (index, _object) in self.inner_layer.objects().enumerate() {
            indexes.push(TileIndex::ObjectIndex(index));
        }
        Box::new(indexes.into_iter())
    }

    fn tile(&self, tile_index: TileIndex) -> Option<Tile> {
        let index = match tile_index {
            TileIndex::ObjectIndex(index) => index,
            _ => return None,
        };

        let object = self.inner_layer.get_object(index)?;
        info!("Object Props: {:?}", object.properties);
        let object_tile = object.get_tile();

        object_tile.and_then(|tile| {
            tile.get_tile().map(|tile_data| {
                let Some(collision) = tile_data.collision.as_ref() else {
                    return Tile {
                        id: tile.id(),
                        collision: None,
                        shapes: vec![],
                    };
                };

                let object_data = collision.object_data();
                let shapes = object_data
                    .iter()
                    .map(|data| match data.shape {
                        tiled_rs::ObjectShape::Rect { width, height } => TileShape::Rect {
                            width,
                            height,
                            x: data.x,
                            y: data.y,
                        },
                        _ => {
                            unimplemented!()
                        }
                    })
                    .collect();

                Tile {
                    id: tile.id(),
                    // Retrieving a custom property named "collision" only if it exists and is a boolean
                    collision: tile_data.properties.get("collision").and_then(|v| {
                        if let tiled_rs::PropertyValue::BoolValue(b) = v {
                            Some(*b)
                        } else {
                            None
                        }
                    }),
                    shapes,
                }
            })
        })

        // object_tile.and_then(|tile| {
        //     tile.
        //     tile.get_tile().map(|tile_data| {
        //         let Some(collision) = tile_data.collision.as_ref() else {
        //             return Tile {
        //                 id: tile.id(),
        //             collision: None,
        //             shapes: vec![],
        //         };
        //     };

        //     let object_data = collision.object_data();
        //     let shapes = object_data
        //         .iter()
        //         .map(|data| match data.shape {
        //             tiled_rs::ObjectShape::Rect { width, height } => TileShape::Rect {
        //                 width,
        //                 height,
        //                 x: data.x,
        //                 y: data.y,
        //             },
        //             _ => {
        //                 unimplemented!()
        //             }
        //         })
        //         .collect();

        //     Tile {
        //         id: tile.id(),
        //         // Retrieving a custom property named "collision" only if it exists and is a boolean
        //         collision: tile_data.properties.get("collision").and_then(|v| {
        //             if let tiled_rs::PropertyValue::BoolValue(b) = v {
        //                 Some(*b)
        //             } else {
        //                 None
        //             }
        //         }),
        //         shapes,
        //     }
        // })          
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
