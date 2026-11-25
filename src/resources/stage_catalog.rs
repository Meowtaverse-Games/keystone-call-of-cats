use crate::resources::chunk_grammar_map::{ChunkGrammarConfig, Map, generate_map_from_config};
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct StageId(pub String);

impl From<&str> for StageId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl std::fmt::Display for StageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMeta {
    pub id: StageId,
    pub published: bool,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(skip)]
    pub category: String,
    #[serde(skip)]
    pub display_number: usize,
}
impl StageMeta {
    pub fn load_map(&self) -> Map {
        let bytes: &[u8] = match self.id.0.as_str() {
            "move-01" => include_bytes!("../../assets/stages/move-01.ron"),
            "move-02" => include_bytes!("../../assets/stages/move-02.ron"),
            "move-03" => include_bytes!("../../assets/stages/move-03.ron"),
            "move-04" => include_bytes!("../../assets/stages/move-04.ron"),
            "move-05" => include_bytes!("../../assets/stages/move-05.ron"),
            "move-06" => include_bytes!("../../assets/stages/move-06.ron"),
            "sense-01" => include_bytes!("../../assets/stages/sense-01.ron"),
            "sense-02" => include_bytes!("../../assets/stages/sense-02.ron"),
            "dig-01" => include_bytes!("../../assets/stages/dig-01.ron"),
            "dig-02" => include_bytes!("../../assets/stages/dig-02.ron"),
            "dig-03" => include_bytes!("../../assets/stages/dig-03.ron"),
            "dig-04" => include_bytes!("../../assets/stages/dig-04.ron"),
            _ => panic!("Stage ID: {} not found", self.id),
        };
        let config: ChunkGrammarConfig =
            ron::de::from_bytes(bytes).unwrap_or_else(|_| panic!("Parse failed: {}.ron", self.id));
        generate_map_from_config(config)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StageCategory {
    pub id: String,
    pub unlock_after: Option<StageId>,
    pub stages: Vec<StageMeta>,
}

#[derive(Resource, Clone, Debug)]
pub struct StageCatalog {
    pub categories: Vec<StageCategory>,
    pub stages: Vec<StageMeta>,
}
impl StageCatalog {
    pub fn new(mut categories: Vec<StageCategory>) -> Self {
        let mut stages = Vec::new();
        for category in &mut categories {
            category
                .stages
                .retain(|stage| stage.published || cfg!(feature = "experimental"));
            category.stages.sort_by_key(|stage| !stage.published);
            for (index, stage) in category.stages.iter_mut().enumerate() {
                stage.category = category.id.clone();
                stage.display_number = index + 1;
                stages.push(stage.clone());
            }
        }
        Self { categories, stages }
    }
    pub fn load_from_assets() -> Self {
        Self::new(ron::from_str(include_str!("../../assets/stages/list.ron")).unwrap())
    }
    pub fn iter(&self) -> impl Iterator<Item = &StageMeta> {
        self.stages.iter()
    }
    pub fn stage_by_index(&self, index: usize) -> Option<&StageMeta> {
        self.stages.get(index)
    }
    pub fn stage_by_id(&self, id: &StageId) -> Option<&StageMeta> {
        self.stages.iter().find(|stage| &stage.id == id)
    }
    pub fn next_stage(&self, id: &StageId) -> Option<&StageMeta> {
        self.stages
            .iter()
            .position(|stage| &stage.id == id)
            .and_then(|i| self.stages.get(i + 1))
    }
    pub fn is_unlocked(&self, id: &StageId, cleared: &std::collections::HashSet<StageId>) -> bool {
        let Some(stage) = self.stage_by_id(id) else {
            return false;
        };
        if stage.unlocked {
            return true;
        }
        let category = self
            .categories
            .iter()
            .find(|cat| cat.id == stage.category)
            .unwrap();
        let previous = if stage.display_number == 1 {
            category.unlock_after.as_ref()
        } else {
            category.stages.get(stage.display_number - 2).map(|s| &s.id)
        };
        previous.is_some_and(|id| self.stage_by_id(id).is_some() && cleared.contains(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, fs, path::Path};
    #[test]
    fn catalog_assets_and_locales_are_complete() {
        let raw: Vec<StageCategory> =
            ron::from_str(include_str!("../../assets/stages/list.ron")).unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut ids = HashSet::new();
        for category in &raw {
            for stage in &category.stages {
                assert!(ids.insert(stage.id.0.clone()));
                let suffix = stage
                    .id
                    .0
                    .strip_prefix(&format!("{}-", category.id))
                    .unwrap();
                assert!(suffix.len() == 2 && suffix.bytes().all(|b| b.is_ascii_digit()));
                assert!(
                    root.join(format!("assets/stages/{}.ron", stage.id))
                        .exists()
                );
                stage.load_map();
            }
        }
        for category in &raw {
            if let Some(gate) = &category.unlock_after {
                assert!(ids.contains(&gate.0));
            }
        }
        for file in fs::read_dir(root.join("assets/stages")).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "ron") && path.file_stem().unwrap() != "list" {
                assert!(ids.contains(path.file_stem().unwrap().to_str().unwrap()));
            }
        }
        for locale in ["ja-JP", "en-US", "zh-Hans"] {
            let text = ["main", "stages"]
                .map(|name| {
                    fs::read_to_string(root.join(format!("assets/locales/{locale}/{name}.ftl")))
                        .unwrap()
                })
                .join("\n");
            for id in &ids {
                for suffix in ["text", "description", "name"] {
                    assert!(
                        text.lines()
                            .any(|l| l.starts_with(&format!("stage-{id}-{suffix} ="))),
                        "{locale}: {id}-{suffix}"
                    );
                }
            }
            for cat in &raw {
                assert!(text.contains(&format!("stage-category-{}-name =", cat.id)));
            }
        }
    }
    #[test]
    fn legacy_mapping_is_frozen() {
        assert_eq!(
            crate::resources::legacy_stage_ids::LEGACY_STAGE_IDS,
            [
                "move-01", "move-02", "move-03", "move-04", "move-05", "move-06", "sense-01",
                "sense-02", "dig-01", "dig-02", "dig-03", "dig-04"
            ]
        );
    }
    #[test]
    fn appending_to_a_completed_category_preserves_in_progress_unlocks() {
        let mut raw: Vec<StageCategory> =
            ron::from_str(include_str!("../../assets/stages/list.ron")).unwrap();
        let cleared = [
            "move-01", "move-02", "move-03", "move-04", "move-05", "move-06", "sense-01",
        ]
        .into_iter()
        .map(StageId::from)
        .collect();
        let before = StageCatalog::new(raw.clone());
        let previously_open: Vec<_> = before
            .iter()
            .filter(|s| before.is_unlocked(&s.id, &cleared))
            .map(|s| s.id.clone())
            .collect();
        let mut added = raw[0].stages[0].clone();
        added.id = "move-07".into();
        added.unlocked = false;
        raw[0].stages.push(added);
        let after = StageCatalog::new(raw);
        for id in previously_open {
            assert!(after.is_unlocked(&id, &cleared));
        }
        assert!(after.is_unlocked(&"move-07".into(), &cleared));
        assert!(!after.is_unlocked(&"dig-01".into(), &cleared));
    }
    #[test]
    fn publication_order_unlocks_and_appended_stages() {
        let mut raw: Vec<StageCategory> =
            ron::from_str(include_str!("../../assets/stages/list.ron")).unwrap();
        let catalog = StageCatalog::new(raw.clone());
        let mut cleared = HashSet::new();
        for (i, stage) in catalog.iter().enumerate() {
            assert!(catalog.is_unlocked(&stage.id, &cleared));
            assert!(
                catalog
                    .iter()
                    .skip(i + 1)
                    .all(|s| s.unlocked || !catalog.is_unlocked(&s.id, &cleared))
            );
            cleared.insert(stage.id.clone());
        }
        assert!(catalog.next_stage(&StageId::from("dig-04")).is_none());
        let mut extra = raw[0].stages[0].clone();
        extra.id = "move-07".into();
        extra.unlocked = false;
        raw[0].stages.push(extra.clone());
        extra.id = "move-08".into();
        extra.published = false;
        raw[0].stages.insert(0, extra);
        let expanded = StageCatalog::new(raw);
        assert!(expanded.is_unlocked(&"sense-01".into(), &cleared));
        assert!(expanded.is_unlocked(&"move-07".into(), &cleared));
        assert_eq!(
            expanded.stage_by_id(&"move-08".into()).is_some(),
            cfg!(feature = "experimental")
        );
        assert!(!expanded.is_unlocked(&"move-08".into(), &cleared));
        cleared.insert("move-07".into());
        assert_eq!(
            expanded.is_unlocked(&"move-08".into(), &cleared),
            cfg!(feature = "experimental")
        );
        assert_eq!(
            expanded.next_stage(&"move-07".into()).unwrap().id.0,
            if cfg!(feature = "experimental") {
                "move-08"
            } else {
                "sense-01"
            }
        );
        if let Some(stage) = expanded.stage_by_id(&"move-08".into()) {
            assert_eq!(stage.display_number, 8);
        }
    }
}
