use std::env;

use bevy::asset::AssetPlugin;
use bevy::{camera::ScalingMode, prelude::*, render::view::ColorGrading};

use bevy_fluent::prelude::*;

use bevy_egui::EguiPlugin;

use avian2d::{debug_render::PhysicsDebugPlugin, prelude::*};

use unic_langid::langid;

mod config;
mod plugins;
mod resources;
mod scenes;
mod systems;
mod util;

use crate::{
    config::*,
    plugins::*,
    resources::{
        chunk_grammar_map,
        game_state::GameState,
        launch_profile::{LaunchProfile, LaunchType},
    },
    scenes::ScenesPlugin,
};

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

fn main() {
    let steam_app_id = steam_app_id();

    let launch_profile = LaunchProfile::from_args(env::args().collect::<Vec<_>>().as_slice());
    if launch_profile.changed {
        println!("Launch profile: {:?}", launch_profile);
    }
    match launch_profile.launch_type {
        LaunchType::ShowChunkGrammarAsciiMap => {
            chunk_grammar_map::show_ascii_map();
            return;
        }
        LaunchType::SteamAppInfo => {
            steam::show_steam_app_info(steam_app_id);
            return;
        }
        _ => {}
    }

    let mut app = App::new();

    app.insert_resource(Locale::new(langid!("ja-JP")).with_default(langid!("en-US")));

    app.add_plugins((
        SteamPlugin::new(steam_app_id),
        StagePlugin,
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
        FluentPlugin,
    ));

    if launch_profile.render_physics {
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
        .add_plugins(TiledPlugin::new("assets/tiled/super-platfomer-assets.tsx"))
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ScenesPlugin)
        .insert_resource(launch_profile)
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
        ColorGrading::default(),
    ));
}
