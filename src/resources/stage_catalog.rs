use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::resources::game_state::TOTAL_STAGE_SLOTS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StageId(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMeta {
    pub id: StageId,
    pub title: String,
}

#[derive(Resource, Clone, Debug)]
pub struct StageCatalog {
    pub stages: Vec<StageMeta>,
}

impl StageCatalog {
    pub fn new(stages: Vec<StageMeta>) -> Self {
        Self { stages }
    }

    pub fn load_from_assets() -> Self {
        Self::new(load_stage_catalog_entries())
    }

    pub fn iter(&self) -> impl Iterator<Item = &StageMeta> {
        self.stages.iter()
    }
}

#[derive(Deserialize)]
struct RonStageEntry {
    title: String,
    #[serde(default)]
    id: Option<usize>,
}

fn load_stage_catalog_entries() -> Vec<StageMeta> {
    const EMBEDDED: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/stages/catalog.ron"
    ));

    if let Ok(entries) = ron::de::from_str::<Vec<RonStageEntry>>(EMBEDDED) {
        return build_stage_meta(entries);
    }

    if let Ok(text) = std::fs::read_to_string("assets/stages/catalog.ron") {
        if let Ok(entries) = ron::de::from_str::<Vec<RonStageEntry>>(&text) {
            return build_stage_meta(entries);
        }
    }

    (0..TOTAL_STAGE_SLOTS)
        .map(|i| StageMeta {
            id: StageId(i),
            title: format!("STAGE {:02}", i + 1),
        })
        .collect()
}

fn build_stage_meta(entries: Vec<RonStageEntry>) -> Vec<StageMeta> {
    entries
        .into_iter()
        .enumerate()
        .map(|(i, entry)| StageMeta {
            id: StageId(entry.id.unwrap_or(i)),
            title: entry.title,
        })
        .collect()
}
