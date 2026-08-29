use crate::resources::{
    file_storage::{FileStorageResource, default_storage},
    stage_catalog::{self, StageCatalog},
    stage_progress::StageProgress,
    stage_scripts::StageScripts,
};
use bevy::prelude::*;
#[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
use std::sync::Arc;

#[cfg(feature = "steam")]
use crate::resources::{file_storage::SteamCloudFileStorage, steam_client::SteamClientResource};

pub fn setup_stage_resources(
    mut commands: Commands,
    #[cfg(feature = "steam")] steam_client: Option<Res<SteamClientResource>>,
    existing_storage: Option<Res<FileStorageResource>>,
    existing_catalog: Option<Res<StageCatalog>>,
    existing_scripts: Option<Res<StageScripts>>,
    existing_progress: Option<Res<StageProgress>>,
) {
    if existing_storage.is_some()
        && existing_catalog.is_some()
        && existing_progress.is_some()
        && existing_scripts.is_some()
    {
        return;
    }

    let storage_backend = {
        #[cfg(all(feature = "steam", not(target_arch = "wasm32")))]
        {
            if let Some(storage) = existing_storage.as_ref() {
                storage.backend()
            } else {
                if let Some(client) = steam_client {
                    let rs = client.remote_storage();
                    if rs.is_cloud_enabled_for_app() && rs.is_cloud_enabled_for_account() {
                        Arc::new(SteamCloudFileStorage::new(&client))
                    } else {
                        default_storage()
                    }
                } else {
                    default_storage()
                }
            }
        }

        #[cfg(any(not(feature = "steam"), target_arch = "wasm32"))]
        {
            existing_storage
                .as_ref()
                .map(|storage| storage.backend())
                .unwrap_or_else(default_storage)
        }
    };

    if existing_storage.is_none() {
        commands.insert_resource(FileStorageResource::new(storage_backend.clone()));
    }

    let stage_catalog_usecase = stage_catalog::StageCatalog::load_from_assets();
    if existing_catalog.is_none() {
        commands.insert_resource(stage_catalog_usecase.clone());
    }

    if existing_scripts.is_none() {
        let scripts = StageScripts::load_or_default(storage_backend.as_ref());
        commands.insert_resource(scripts);
    }

    if existing_progress.is_none() {
        let progress =
            StageProgress::load_or_default(&stage_catalog_usecase, storage_backend.as_ref());
        commands.insert_resource(progress);
    }
}
