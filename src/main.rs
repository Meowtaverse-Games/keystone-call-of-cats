// Hide console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

use crate::resources::stage_catalog::StageId;
use crate::{
    config::*,
    plugins::*,
    resources::{
        chunk_grammar_map,
        game_state::GameState,
        launch_profile::{LaunchProfile, LaunchType},
        settings::GameSettings,
    },
    scenes::ScenesPlugin,
};

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

fn main() {
    #[allow(unused_variables)]
    let steam_app_id = steam_app_id();

    let launch_profile = LaunchProfile::from_args(env::args().collect::<Vec<_>>().as_slice());
    if launch_profile.changed {
        println!("Launch profile: {:?}", launch_profile);
    }
    match launch_profile.launch_type {
        LaunchType::ShowChunkGrammarAsciiMap => {
            chunk_grammar_map::show_ascii_map(launch_profile.stage_id.unwrap_or(StageId(1)).0);
            return;
        }
        #[cfg(feature = "steam")]
        LaunchType::SteamAppInfo => {
            steam::show_steam_app_info(steam_app_id);
            return;
        }
        _ => {}
    }

    let mut app = App::new();

    let storage = resources::file_storage::LocalFileStorage::default_dir();
    let mut settings = GameSettings::load_or_default(&storage);

    let locale_id = if let Some(saved_locale) = &settings.locale {
        saved_locale.parse().unwrap_or_else(|_| langid!("en-US"))
    } else {
        let determined = determine_initial_locale();
        settings.locale = Some(determined.to_string());
        if let Err(e) = settings.persist(&storage) {
            warn!("Failed to persist determined locale: {}", e);
        }
        determined
    };

    app.insert_resource(Locale::new(locale_id).with_default(langid!("en-US")))
        .insert_resource(launch_profile.clone())
        .add_systems(
            OnEnter(GameState::Reloading),
            |mut next_state: ResMut<NextState<GameState>>| {
                next_state.set(GameState::SelectStage);
            },
        );

    app.add_plugins((
        #[cfg(feature = "steam")]
        SteamPlugin::new(steam_app_id),
        StagePlugin,
        SettingsPlugin,
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
            1800.0,
            1200.0,
            Color::linear_rgb(0.0, 0.0, 0.0),
        ))
        .add_plugins(TiledPlugin::new("images/spa.png"))
        .add_plugins(AssetLoaderPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ScenesPlugin)
        .insert_resource(launch_profile)
        .init_resource::<resources::stone_type::StoneCapabilities>()
        .init_state::<GameState>()
        .run();
}

fn determine_initial_locale() -> unic_langid::LanguageIdentifier {
    // Priority: ITCHIO_OFFICIAL_LOCALE > LANG > System Default (if accessible via some crate, but std::env is easier)
    let env_locale = std::env::var("ITCHIO_OFFICIAL_LOCALE")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en-US".to_string());

    let short_code = env_locale.split(&['-', '_'][..]).next().unwrap_or("en");

    match short_code {
        "ja" => langid!("ja-JP"),
        "zh" => langid!("zh-Hans"), // Assuming Simplified Chinese for generic 'zh'
        _ => langid!("en-US"),
    }
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
