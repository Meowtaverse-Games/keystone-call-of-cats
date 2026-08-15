use crate::resources::{
    file_storage::{FileError, FileStorage},
    script_engine::Language,
    stage_catalog::StageId,
};
use bevy::prelude::{Resource, info, warn};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

pub const STAGE_SCRIPTS_FILE: &str = "stage_scripts.ron";

/// Stores the latest editor script per stage.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageScripts {
    #[serde(deserialize_with = "deserialize_scripts_compat")]
    scripts: HashMap<Language, HashMap<StageId, Vec<String>>>,
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

    pub fn stage_codes(&self, lang: Language, stage_id: StageId) -> Option<&[String]> {
        self.scripts
            .get(&lang)?
            .get(&stage_id)
            .map(|v| v.as_slice())
    }

    pub fn set_stage_codes(&mut self, lang: Language, stage_id: StageId, codes: Vec<String>) {
        self.scripts
            .entry(lang)
            .or_default()
            .insert(stage_id, codes);
    }
}

fn deserialize_scripts_compat<'de, D>(
    deserializer: D,
) -> Result<HashMap<Language, HashMap<StageId, Vec<String>>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Single(String),
        Multiple(Vec<String>),
    }

    let raw: HashMap<Language, HashMap<StageId, StringOrVec>> = HashMap::deserialize(deserializer)?;
    let mut result = HashMap::new();

    for (lang, stages) in raw {
        let mut stage_map = HashMap::new();
        for (stage_id, value) in stages {
            let vec_value = match value {
                StringOrVec::Single(s) => vec![s],
                StringOrVec::Multiple(v) => v,
            };
            stage_map.insert(stage_id, vec_value);
        }
        result.insert(lang, stage_map);
    }

    Ok(result)
}
