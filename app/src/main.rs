use bevy::prelude::*;

use keystone_cc_adapter::assets_loader::AssetsLoaderPlugin;
use keystone_cc_adapter::VisibilityPlugin;
use keystone_cc_adapter::{game_state::GameState, CameraPlugin};
use keystone_cc_scenes::ScenesPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VisibilityPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(AssetsLoaderPlugin)
        .add_plugins(ScenesPlugin)
        .init_state::<GameState>()
        .run();
}
