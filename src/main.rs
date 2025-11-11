use std::env;

use bevy::asset::AssetPlugin;
use bevy::{camera::ScalingMode, prelude::*};

use bevy_egui::EguiPlugin;

use avian2d::debug_render::PhysicsDebugPlugin;
use avian2d::prelude::*;

mod application;
mod config;
mod domain;
mod infrastructure;
mod presentation;

use crate::config::*;

use crate::application::ports::StageRepository;
use crate::application::usecase::stage_progress_usecase::StageProgressServiceRes;
use crate::application::*;
use crate::infrastructure::*;
use crate::presentation::ScenesPlugin;

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
        LaunchType::GenerateChunkGrammerMap => {
            domain::chunk_grammar_map::main();
            return;
        }
        LaunchType::SteamAppInfo => {
            show_steam_app_info(steam_app_id);
            return;
        }
        _ => {}
    }

    let mut app = App::new();

    app.add_plugins(SteamPlugin::new(steam_app_id))
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
        .add_systems(Startup, setup_file_storage)
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
    ));
}

// ---------- Composition Root: FileStorage DI ----------
use crate::application::ports::file_storage::FileStorage;
use std::sync::Arc;

#[derive(Resource, Clone)]
pub struct FileStorageRes(pub Arc<dyn FileStorage + Send + Sync>);

#[derive(Resource, Clone)]
pub struct StageRepositoryRes(pub Arc<dyn StageRepository + Send + Sync>);

fn setup_file_storage(mut commands: Commands, steam_client: Option<Res<SteamClient>>) {
    use crate::application::game_state::TOTAL_STAGE_SLOTS;
    use crate::infrastructure::storage::local_file_storage::LocalFileStorage;
    use crate::infrastructure::storage::steam_cloud_file_storage::SteamCloudFileStorage;
    use std::path::Path;

    let storage: Arc<dyn FileStorage + Send + Sync> = if let Some(client) = steam_client {
        let rs = client.remote_storage();
        if rs.is_cloud_enabled_for_app() && rs.is_cloud_enabled_for_account() {
            Arc::new(SteamCloudFileStorage::new(client.clone()))
        } else {
            Arc::new(LocalFileStorage::default_dir())
        }
    } else {
        Arc::new(LocalFileStorage::default_dir())
    };

    let storage_res = FileStorageRes(storage);
    commands.insert_resource(storage_res.clone());

    commands.insert_resource(StageProgressServiceRes::new(storage_res.0.clone()));

    let repo: Arc<dyn StageRepository + Send + Sync> = match EmbeddedStageRepository::load() {
        Ok(r) => Arc::new(r),
        Err(err) => {
            warn!(
                "Stage catalog: embedded RON failed to parse: {}. Trying filesystem.",
                err
            );
            match FileStageRepository::load_from(Path::new("assets/stages/catalog.ron")) {
                Ok(file_repo) => Arc::new(file_repo),
                Err(err) => {
                    warn!(
                        "Stage catalog: failed to load RON file, falling back to static: {}",
                        err
                    );
                    Arc::new(StaticStageRepository::new(TOTAL_STAGE_SLOTS))
                }
            }
        }
    };
    commands.insert_resource(StageRepositoryRes(repo));
}
