use bevy::prelude::Vec2;
use tiled::{self as tiled_rs};

#[derive(Debug)]
pub struct Object {
    pub id: u32,
    pub position: Vec2,
}

pub struct ObjectLayer<'map> {
    pub name: String,
    pub inner_layer: tiled_rs::ObjectLayer<'map>,
}

impl<'map> ObjectLayer<'map> {
    pub(super) fn new(name: String, object_layer: tiled_rs::ObjectLayer<'map>) -> Self {
        Self {
            name,
            inner_layer: object_layer,
        }
    }
}

impl<'map> ObjectLayer<'map> {
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn object_indexes(&self) -> Vec<usize> {
        (0..self.inner_layer.objects().count()).collect()
    }

    pub fn object(&self, index: usize) -> Object {
        let object = self.inner_layer.get_object(index).unwrap();
        let tile_data = object.tile_data().unwrap();

        Object {
            id: tile_data.id(),
            position: Vec2::new(object.x, object.y),
        }
    }
}
