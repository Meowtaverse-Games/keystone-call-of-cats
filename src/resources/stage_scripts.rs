use bevy::prelude::{Resource, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::resources::{
    file_storage::{FileError, FileStorage},
    stage_catalog::StageId,
};

pub const STAGE_SCRIPTS_FILE: &str = "stage_scripts.ron";

/// Stores the latest editor script per stage.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageScripts {
    scripts: HashMap<StageId, String>,
}

impl StageScripts {
    pub fn load_or_default(storage: &dyn FileStorage) -> Self {
        match storage.load(STAGE_SCRIPTS_FILE) {
            Ok(Some(bytes)) => ron::de::from_bytes(&bytes).unwrap_or_else(|err| {
                warn!("Failed to parse saved stage scripts: {err}");
                StageScripts::default()
            }),
            Ok(None) => StageScripts::default(),
            Err(err) => {
                warn!("Failed to load saved stage scripts: {err}");
                StageScripts::default()
            }
        }
    }

    pub fn persist(&self, storage: &dyn FileStorage) -> Result<(), FileError> {
        let serialized = ron::ser::to_string(self)
            .map_err(|err| FileError::Other(format!("serialize stage scripts: {err}")))?;
        info!("Saving stage scripts ({} entries)", self.scripts.len());
        storage
            .save(STAGE_SCRIPTS_FILE, serialized.as_bytes())
            .map_err(|err| {
                warn!("Failed to save stage scripts: {err}");
                err
            })
    }

    pub fn stage_code(&self, stage_id: StageId) -> Option<&str> {
        self.scripts.get(&stage_id).map(String::as_str)
    }

    pub fn set_stage_code(&mut self, stage_id: StageId, code: String) {
        self.scripts.insert(stage_id, code);
    }
}
