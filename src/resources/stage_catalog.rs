use bevy::prelude::{Resource, *};
use serde::{Deserialize, Serialize};

use crate::resources::chunk_grammar_map::{ChunkGrammarConfig, Map, generate_map_from_config};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct StageId(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMeta {
    pub id: StageId,
    pub title: String,
    pub unlocked: bool,
}

impl StageMeta {
    pub fn load_map(&self) -> Map {
        let stage_id = self.id.0;
        let bytes: &'static [u8] = match stage_id {
            1 => include_bytes!("../../assets/stages/stage-1.ron"),
            2 => include_bytes!("../../assets/stages/stage-2.ron"),
            3 => include_bytes!("../../assets/stages/stage-3.ron"),
            4 => include_bytes!("../../assets/stages/stage-4.ron"),
            5 => include_bytes!("../../assets/stages/stage-5.ron"),
            6 => include_bytes!("../../assets/stages/stage-6.ron"),
            7 => include_bytes!("../../assets/stages/stage-7.ron"),
            8 => include_bytes!("../../assets/stages/stage-8.ron"),
            9 => include_bytes!("../../assets/stages/stage-9.ron"),
            10 => include_bytes!("../../assets/stages/stage-10.ron"),
            11 => include_bytes!("../../assets/stages/stage-11.ron"),
            12 => include_bytes!("../../assets/stages/stage-12.ron"),
            18 => include_bytes!("../../assets/stages/stage-18.ron"),
            _ => panic!("Stage ID: {} Not found.", stage_id),
        };

        let config: ChunkGrammarConfig = ron::de::from_bytes(bytes)
            .unwrap_or_else(|_| panic!("Parse failed: stage-{}.ron", stage_id));

        generate_map_from_config(config)
    }
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

    pub fn stage_by_index(&self, index: usize) -> Option<&StageMeta> {
        self.stages.get(index)
    }

    pub fn stage_by_id(&self, stage_id: StageId) -> Option<&StageMeta> {
        self.stages.iter().find(|stage| stage.id == stage_id)
    }

    pub fn next_stage(&self, stage_id: StageId) -> Option<&StageMeta> {
        let next_stage_id = StageId(stage_id.0 + 1);
        self.stages.iter().find(|stage| stage.id == next_stage_id)
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
            title: format!("Stage {}", entry.id),
            unlocked: entry.unlocked,
        })
        .collect()
}
