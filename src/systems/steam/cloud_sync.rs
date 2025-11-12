use bevy::prelude::*;

use crate::resources::{
    file_storage::FileStorageResource, stage_progress::StageProgress,
    steam_client::SteamClientResource,
};

/// Placeholder for Steam Cloud sync.
pub fn sync_cloud_save_system(
    _steam: Res<SteamClientResource>,
    _storage: Option<Res<FileStorageResource>>,
    _progress: Option<Res<StageProgress>>,
) {
    // Future: compare local saves with Steam Cloud and synchronize.
}
