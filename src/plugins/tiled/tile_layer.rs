use tiled::{self as tiled_rs};

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
    pub id: u32,
    #[allow(dead_code)]
    pub collision: Option<bool>,
    pub shapes: Vec<TileShape>,
}

pub struct TileLayer<'map> {
    pub name: String,
    pub inner_layer: tiled_rs::TileLayer<'map>,
}

impl<'map> TileLayer<'map> {
    pub(super) fn new(name: String, tile_layer: tiled_rs::TileLayer<'map>) -> Self {
        Self {
            name,
            inner_layer: tile_layer,
        }
    }
}

impl<'map> TileLayer<'map> {
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn width(&self) -> u32 {
        self.inner_layer.width().unwrap_or(0)
    }

    pub fn height(&self) -> u32 {
        self.inner_layer.height().unwrap_or(0)
    }

    pub fn tile_positions(&self) -> Vec<(u32, u32)> {
        let mut indexes = vec![(0u32, 0u32)];
        for x in 0..self.inner_layer.width().unwrap() as i32 {
            for y in 0..self.inner_layer.height().unwrap() as i32 {
                if let Some(_tile) = self.inner_layer.get_tile(x, y) {
                    indexes.push((x as u32, y as u32));
                }
            }
        }
        indexes
    }

    pub fn tile(&self, x: u32, y: u32) -> Option<Tile> {
        let tile = self.inner_layer.get_tile(x as i32, y as i32)?;

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
