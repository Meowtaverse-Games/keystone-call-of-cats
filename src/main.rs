mod adapter;
mod core;
mod plugins;
mod scenes;

use bevy::prelude::*;
use bevy::asset::AssetPlugin;

use bevy_egui::EguiPlugin;
use tiled::Loader;

use crate::adapter::{VisibilityPlugin, game_state::GameState};
use crate::plugins::DesignResolutionPlugin;
use crate::plugins::assets_loader::AssetLoaderPlugin;
use crate::scenes::ScenesPlugin;

fn main() {
    let mut loader = Loader::new();
    let map = loader.load_tmx_map("assets/tiled/stage1-1.tmx").unwrap();
    println!("{:?}", map);
    println!("{:?}", map.tilesets()[0].get_tile(0).unwrap().probability);

    let tileset = loader.load_tsx_tileset("assets/tiled/super-platfomer-assets.tsx").unwrap();
    assert_eq!(*map.tilesets()[0], tileset);

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "assets".to_string(),
                    watch_for_changes_override: Some(true),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "keystone: call of cats".to_string(),
                        visible: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(VisibilityPlugin)
        .add_plugins(
            DesignResolutionPlugin::new(1600.0, 1200.0, Color::linear_rgb(0.02, 0.02, 0.02))
                .fix_min(800.0 * 2.0, 600.0),
        )
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ScenesPlugin)
        .init_state::<GameState>()
        .run();
}
