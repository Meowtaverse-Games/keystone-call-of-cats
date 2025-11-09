use std::env;

mod application;
mod domain;
mod infrastructure;
mod presentation;

use bevy::asset::AssetPlugin;
use bevy::{camera::ScalingMode, prelude::*};

use bevy_egui::EguiPlugin;

use avian2d::debug_render::PhysicsDebugPlugin;
use avian2d::prelude::*;

use crate::application::{GameState, Mode, STEAM_APP_ID};
use crate::infrastructure::engine::{
    AssetLoaderPlugin, DesignResolutionPlugin, ScriptPlugin, SteamPlugin, TiledPlugin,
    VisibilityPlugin,
};
use crate::presentation::ScenesPlugin;

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--chunk-grammar-map" => {
                domain::chunk_grammar_map::main();
                return;
            }
            "--steam-test" => {
                infrastructure::steam::show_steam_app_info(STEAM_APP_ID);
                return;
            }
            _ => {}
        }
    }

    let mode = Mode::from_args(&args);
    if mode.changed {
        println!("Operating mode: {:?}", mode);
    }

    let mut app = App::new();

    app.add_plugins(SteamPlugin::new(STEAM_APP_ID))
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
        ));

    if mode.render_physics {
        app.add_plugins(PhysicsDebugPlugin);
    }

    app.add_plugins(ScriptPlugin)
        .add_plugins(VisibilityPlugin)
        .add_systems(Startup, setup_camera)
        .add_plugins(DesignResolutionPlugin::new(
            1600.0,
            1200.0,
            Color::linear_rgb(0.0, 0.0, 0.0),
        ))
        .add_plugins(TiledPlugin::new(
            vec![
                "assets/tiled/stage1-1.tmx".to_string(),
                "assets/tiled/stage1-2.tmx".to_string(),
            ],
            "assets/tiled/super-platfomer-assets.tsx",
        ))
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ScenesPlugin)
        .insert_resource(mode)
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
