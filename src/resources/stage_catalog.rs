use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

#[derive(Deserialize)]
struct RonStageEntry {
    title: String,
    #[serde(default)]
    unlocked: Option<bool>,
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
        .enumerate()
        .map(|(i, entry)| StageMeta {
            id: StageId(i),
            title: entry.title,
            unlocked: entry.unlocked.unwrap_or(false),
        })
        .collect()
}
