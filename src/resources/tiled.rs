// use std::sync::Arc;

use bevy::{math::UVec2, prelude::*};

#[derive(Resource, Clone)]
pub struct TiledLoaderConfig {
    pub image_path: String,
}

impl TiledLoaderConfig {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            image_path: path.into(),
        }
    }
}

#[derive(Resource, Clone)]
pub struct TiledMapAssets {
    pub tileset: Tileset,
}

impl TiledMapAssets {
    pub fn tile(&self, id: u32) -> Option<Tile> {
        let shapes = if id == 235 || id == 236 {
            vec![TileShape::Rect {
                width: 16.0,
                height: 9.0,
                x: 0.0,
                y: 3.0,
            }]
        } else {
            vec![]
        };

        Some(Tile { shapes })
    }

    pub fn map_size(&self) -> Vec2 {
        Vec2::new(MAP_SIZE.0 as f32, MAP_SIZE.1 as f32)
    }

    pub fn tile_size(&self) -> Vec2 {
        Vec2::new(TILE_SIZE.0, TILE_SIZE.1)
    }

    pub fn map_pixel_size(&self, tile_size: Vec2) -> Vec2 {
        let map_size = self.map_size();
        Vec2::new(map_size.x * tile_size.x, map_size.y * tile_size.y)
    }

    pub fn scaled_tile_size_and_scale(&self, viewport_size: Vec2, tile_size: Vec2) -> (Vec2, f32) {
        let scale = (viewport_size / self.map_pixel_size(tile_size)).min_element();
        (tile_size * scale, scale)
    }
}

pub const MAP_SIZE: (usize, usize) = (30, 20);
pub const TILE_SIZE: (f32, f32) = (16.0, 16.0);

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
pub struct Tile {
    pub shapes: Vec<TileShape>,
}

#[derive(Clone)]
pub struct Tileset {
    pub image: Option<TiledTilesetImage>,
}

impl Tileset {
    pub fn image(&self) -> Option<&TiledTilesetImage> {
        self.image.as_ref()
    }

    pub fn atlas_sprite(&self, local_id: u32) -> Option<TiledAtlasSprite> {
        let image = self.image.as_ref()?;

        Some(TiledAtlasSprite {
            texture: image.texture.clone(),
            atlas: TextureAtlas {
                layout: image.layout.clone(),
                index: local_id as usize,
            },
        })
    }

    pub fn tile_size(&self) -> Vec2 {
        let raw_tile_size = self
            .image()
            .map(|image| image.tile_size)
            .expect("Failed to get tile size from tileset image");

        Vec2::new(raw_tile_size.x as f32, raw_tile_size.y as f32)
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
