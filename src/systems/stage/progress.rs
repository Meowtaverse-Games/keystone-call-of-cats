use bevy::prelude::*;

use crate::resources::{file_storage::FileStorageResource, stage_progress::StageProgress};

pub fn persist_stage_progress(progress: Res<StageProgress>, storage: Res<FileStorageResource>) {
    if !progress.is_changed() {
        return;
    }

    if let Err(err) = progress.persist(storage.backend().as_ref()) {
        warn!("Failed to persist stage progress: {err}");
    }
}
