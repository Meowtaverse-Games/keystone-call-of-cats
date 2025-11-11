use bevy::prelude::*;
use std::sync::Arc;

use crate::resources::{
    file_storage::{FileStorage, FileStorageResource, LocalFileStorage, SteamCloudFileStorage},
    stage_catalog::StageCatalog,
    stage_progress::StageProgress,
    steam_client::SteamClientResource,
};

pub fn setup_stage_resources(
    mut commands: Commands,
    steam_client: Option<Res<SteamClientResource>>,
    existing_storage: Option<Res<FileStorageResource>>,
    existing_catalog: Option<Res<StageCatalog>>,
    existing_progress: Option<Res<StageProgress>>,
) {
    if existing_storage.is_some() && existing_catalog.is_some() && existing_progress.is_some() {
        return;
    }

    let storage_backend: Arc<dyn FileStorage + Send + Sync> = if let Some(client) = steam_client {
        let rs = client.remote_storage();
        if rs.is_cloud_enabled_for_app() && rs.is_cloud_enabled_for_account() {
            Arc::new(SteamCloudFileStorage::new(&client))
        } else {
            Arc::new(LocalFileStorage::default_dir())
        }
    } else {
        Arc::new(LocalFileStorage::default_dir())
    };

    if existing_storage.is_none() {
        commands.insert_resource(FileStorageResource::new(storage_backend.clone()));
    }

    if existing_catalog.is_none() {
        commands.insert_resource(StageCatalog::load_from_assets());
    }

    if existing_progress.is_none() {
        let progress = StageProgress::load_or_default(storage_backend.as_ref());
        commands.insert_resource(progress);
    }
}
