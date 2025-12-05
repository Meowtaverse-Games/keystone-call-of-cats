use bevy::{app::AppExit, prelude::MessageReader, prelude::*};

use crate::resources::{file_storage::FileStorageResource, stage_scripts::StageScripts};

pub fn persist_stage_scripts(
    scripts: Res<StageScripts>,
    storage: Option<Res<FileStorageResource>>,
) {
    if !scripts.is_changed() {
        return;
    }

    let Some(storage) = storage else {
        return;
    };

    if let Err(err) = scripts.persist(storage.backend().as_ref()) {
        warn!("Failed to persist stage scripts: {err}");
    }
}

pub fn persist_stage_scripts_on_app_exit(
    scripts: Option<Res<StageScripts>>,
    storage: Option<Res<FileStorageResource>>,
    exit_events: MessageReader<AppExit>,
) {
    if exit_events.is_empty() {
        return;
    }
    let (Some(scripts), Some(storage)) = (scripts, storage) else {
        return;
    };

    if let Err(err) = scripts.persist(storage.backend().as_ref()) {
        warn!("Failed to persist stage scripts on exit: {err}");
    }
}
