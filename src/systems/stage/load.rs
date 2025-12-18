use bevy::prelude::*;
use std::sync::Arc;

use crate::resources::{
    file_storage::{FileStorage, FileStorageResource, LocalFileStorage},
    stage_catalog::{self, StageCatalog},
    stage_progress::StageProgress,
    stage_scripts::StageScripts,
};

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

    let storage_backend: Arc<dyn FileStorage + Send + Sync>;

    #[cfg(feature = "steam")]
    {
        storage_backend = if let Some(client) = steam_client {
            let rs = client.remote_storage();
            if rs.is_cloud_enabled_for_app() && rs.is_cloud_enabled_for_account() {
                Arc::new(SteamCloudFileStorage::new(&client))
            } else {
                Arc::new(LocalFileStorage::default_dir())
            }
        } else {
            Arc::new(LocalFileStorage::default_dir())
        };
    }

    #[cfg(not(feature = "steam"))]
    {
        storage_backend = Arc::new(LocalFileStorage::default_dir());
    }

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
