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

    if let Some(storage) = existing_storage.as_deref() {
        storage_backend = storage_backend_from_existing(storage);
    } else {
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

fn storage_backend_from_existing(
    storage: &FileStorageResource,
) -> Arc<dyn FileStorage + Send + Sync> {
    storage.backend()
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use crate::resources::file_storage::{FileStorage, FileStorageResource, LocalFileStorage};

    use super::storage_backend_from_existing;

    #[test]
    fn supplied_storage_is_used_instead_of_a_separate_default_directory() {
        let root =
            std::env::temp_dir().join(format!("keystone-storage-test-{}", std::process::id()));
        let normal = LocalFileStorage::new(root.join("normal"));
        let isolated = LocalFileStorage::new(root.join("isolated"));
        normal
            .save("stage_scripts.ron", b"real-user-sentinel")
            .unwrap();

        let supplied = FileStorageResource::new(Arc::new(isolated));
        let selected = storage_backend_from_existing(&supplied);

        assert_eq!(selected.load("stage_scripts.ron").unwrap(), None);
        assert_eq!(
            normal.load("stage_scripts.ron").unwrap(),
            Some(b"real-user-sentinel".to_vec())
        );
        fs::remove_dir_all(root).unwrap();
    }
}
