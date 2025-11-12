use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct StageId(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMeta {
    pub id: StageId,
    pub title: String,
    pub unlocked: bool,
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

    pub fn max_unlocked_stage_id(&self) -> StageId {
        self.stages
            .iter()
            .filter(|stage| stage.unlocked)
            .map(|stage| stage.id)
            .max_by_key(|id| id.0)
            .unwrap_or_default()
    }
}

#[derive(Deserialize, Default)]
struct RonStageEntry {
    id: usize,
    title: String,
    #[serde(default)]
    unlocked: bool,
}

fn load_stage_catalog_entries() -> Vec<StageMeta> {
    const EMBEDDED: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/stages/list.ron"
    ));

    let entries = ron::de::from_str::<Vec<RonStageEntry>>(EMBEDDED).unwrap();
    build_stage_meta(entries)
}

fn build_stage_meta(entries: Vec<RonStageEntry>) -> Vec<StageMeta> {
    entries
        .into_iter()
        .map(|entry| StageMeta {
            id: StageId(entry.id),
            title: entry.title,
            unlocked: entry.unlocked,
        })
        .collect()
}
