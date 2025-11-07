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
#[allow(dead_code)]
pub struct Tile {
    pub id: u32,
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

    #[allow(dead_code)]
    pub fn tile(&self, x: u32, y: u32) -> Option<Tile> {
        let tile = self.inner_layer.get_tile(x as i32, y as i32)?;

        tile.get_tile().map(|tile_data| {
            let Some(collision) = tile_data.collision.as_ref() else {
                return Tile {
                    id: tile.id(),
                    shapes: vec![],
                };
            };

            let shapes = collision
                .object_data()
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
                shapes,
            }
        })
    }
}
