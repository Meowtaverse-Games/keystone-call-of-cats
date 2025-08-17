use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use keystone_cc_adapter::{VisibilityPlugin, game_state::GameState};
use keystone_cc_plugins::DesignResolutionPlugin;
use keystone_cc_plugins::assets_loader::AssetLoaderPlugin;
use keystone_cc_scenes::ScenesPlugin;

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
        .add_plugins(DesignResolutionPlugin::new(1600.0, 1200.0).fix_min(800.0, 600.0))
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(ScenesPlugin)
        .init_state::<GameState>()
        .run();
}
