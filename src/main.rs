mod adapter;
mod core;
mod plugins;
mod scenes;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use bevy_egui::EguiPlugin;

use crate::adapter::{VisibilityPlugin, game_state::GameState};
use crate::plugins::*;
use crate::scenes::ScenesPlugin;

fn main() {
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
        .add_plugins(TiledPlugin::new("assets/tiled/stage1-1.tmx"))
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ScenesPlugin)
        .init_state::<GameState>()
        .run();
}
