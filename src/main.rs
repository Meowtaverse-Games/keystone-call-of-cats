mod adapter;
mod core;
mod plugins;
mod scenes;

use bevy::asset::AssetPlugin;
use bevy::{camera::ScalingMode, prelude::*};

use bevy_egui::EguiPlugin;

use avian2d::debug_render::PhysicsDebugPlugin;
use avian2d::prelude::*;

use crate::adapter::game_state::GameState;
use crate::plugins::*;
use crate::scenes::ScenesPlugin;

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

fn main() {
    App::new()
        .add_plugins((
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
                })
                .set(ImagePlugin::default_nearest()),
            PhysicsPlugins::default(),
            PhysicsDebugPlugin,
        ))
        .add_plugins(ScriptPlugin)
        .add_plugins(VisibilityPlugin)
        .add_systems(Startup, setup_camera)
        .add_plugins(DesignResolutionPlugin::new(
            1600.0,
            1200.0,
            Color::linear_rgb(0.02, 0.02, 0.02),
        ))
        .add_plugins(TiledPlugin::new(
            "assets/tiled/stage1-1.tmx",
            "assets/tiled/super-platfomer-assets.tsx",
        ))
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ScenesPlugin)
        .init_state::<GameState>()
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            ..OrthographicProjection::default_2d()
        }),
    ));
}
