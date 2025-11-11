use bevy::prelude::*;

use crate::{
    resources::tiled::TiledLoaderConfig, systems::engine::tiled_loader::load_tiled_assets,
};

pub struct TiledPlugin {
    config: TiledLoaderConfig,
}

impl TiledPlugin {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            config: TiledLoaderConfig::new(path),
        }
    }
}

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .add_systems(Startup, load_tiled_assets);
    }
}
