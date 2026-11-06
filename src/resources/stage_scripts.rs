use crate::resources::{
    file_storage::{FileError, FileStorage, backup},
    legacy_stage_ids::legacy_stage_id,
    script_engine::Language,
    stage_catalog::StageId,
};
use bevy::prelude::{Resource, info, warn};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

pub const STAGE_SCRIPTS_FILE: &str = "stage_scripts.ron";
type CodeMap<K> = HashMap<Language, HashMap<K, Vec<String>>>;
type StageCodeMap = CodeMap<StageId>;

/// Stores the latest editor script per stage.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageScripts {
    #[serde(deserialize_with = "deserialize_scripts_compat")]
    scripts: StageCodeMap,
    #[serde(skip)]
    write_blocked: bool,
}

impl StageScripts {
    pub fn load_or_default(storage: &dyn FileStorage) -> Self {
        let bytes = match storage.load(STAGE_SCRIPTS_FILE) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Self::default(),
            Err(err) => {
                warn!("Failed to load stage scripts; writes disabled: {err}");
                return Self {
                    write_blocked: true,
                    ..Self::default()
                };
            }
        };
        if let Ok(current) = ron::de::from_bytes::<Self>(&bytes) {
            return current;
        }
        let preserved = backup(storage, STAGE_SCRIPTS_FILE, &bytes);
        if let Err(err) = &preserved {
            warn!("Cannot preserve stage scripts; writes disabled: {err}");
        }
        let Ok(old) = ron::de::from_bytes::<LegacyScripts>(&bytes) else {
            warn!("Unreadable stage scripts; using defaults after backup attempt");
            return Self {
                write_blocked: preserved.is_err(),
                ..Self::default()
            };
        };
        let mut result = Self {
            write_blocked: preserved.is_err(),
            ..Self::default()
        };
        let mut unmapped = LegacyScripts::default();
        let mut count = 0;
        for (language, stages) in old.scripts {
            for (number, codes) in stages {
                if let Some(id) = legacy_stage_id(number.0) {
                    count += codes.len();
                    result.set_stage_codes(language, id.into(), codes);
                } else {
                    info!("Preserving unmapped scripts for old stage {}", number.0);
                    unmapped
                        .scripts
                        .entry(language)
                        .or_default()
                        .insert(number, codes);
                }
            }
        }
        // Preserve unmapped scripts before replacing the source. A failed sidecar write blocks migration.
        if !unmapped.scripts.is_empty() {
            let saved = (|| {
                let name = "stage_scripts.legacy-unmapped.ron";
                if let Some(previous) = storage.load(name)? {
                    backup(storage, name, &previous)?;
                }
                let text =
                    ron::ser::to_string(&unmapped).map_err(|e| FileError::Other(e.to_string()))?;
                storage.save(name, text.as_bytes())
            })();
            if let Err(err) = saved {
                warn!("Cannot preserve unmapped scripts: {err}");
                result.write_blocked = true;
            }
        }
        if let Err(err) = result.persist(storage) {
            warn!("Scripts migration save failed; retry next launch: {err}");
            result.write_blocked = true;
        } else {
            info!("Migrated {count} stone scripts to stable stage IDs");
        }
        result
    }

    pub fn persist(&self, storage: &dyn FileStorage) -> Result<(), FileError> {
        if self.write_blocked {
            return Err(FileError::Other(
                "stage scripts writes blocked to preserve original data".into(),
            ));
        }
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

    pub fn stage_codes(&self, lang: Language, stage_id: &StageId) -> Option<&[String]> {
        self.scripts.get(&lang)?.get(stage_id).map(|v| v.as_slice())
    }

    pub fn set_stage_codes(&mut self, lang: Language, stage_id: StageId, codes: Vec<String>) {
        self.scripts
            .entry(lang)
            .or_default()
            .insert(stage_id, codes);
    }
}

fn deserialize_scripts_compat<'de, D, K>(deserializer: D) -> Result<CodeMap<K>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + std::hash::Hash,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Single(String),
        Multiple(Vec<String>),
    }

    let raw: HashMap<Language, HashMap<K, StringOrVec>> = HashMap::deserialize(deserializer)?;
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

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct LegacyId(usize);
#[derive(Default, Serialize, Deserialize)]
struct LegacyScripts {
    #[serde(deserialize_with = "deserialize_scripts_compat")]
    scripts: CodeMap<LegacyId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::file_storage::test_storage::MemoryStorage;
    #[test]
    fn unreadable_storage_never_allows_default_overwrites() {
        let storage = MemoryStorage::default();
        storage.save(STAGE_SCRIPTS_FILE, b"original").unwrap();
        *storage.fail_load.borrow_mut() = true;
        let scripts = StageScripts::load_or_default(&storage);
        *storage.fail_load.borrow_mut() = false;
        assert!(scripts.persist(&storage).is_err());
        assert_eq!(
            storage.load(STAGE_SCRIPTS_FILE).unwrap().unwrap(),
            b"original"
        );
    }
    #[test]
    fn migrates_multilanguage_strings_vectors_and_unmapped_without_loss() {
        let storage = MemoryStorage::default();
        let bytes = br#"(scripts:{Rhai:{(1):"line1\nline2",(7):["a","b"],(99):["keep"]},Keystone:{(9):["dig"]}})"#;
        storage.save(STAGE_SCRIPTS_FILE, bytes).unwrap();
        let scripts = StageScripts::load_or_default(&storage);
        assert_eq!(
            scripts
                .stage_codes(Language::Rhai, &"move-01".into())
                .unwrap(),
            &["line1\nline2"]
        );
        assert_eq!(
            scripts
                .stage_codes(Language::Rhai, &"sense-01".into())
                .unwrap(),
            &["a", "b"]
        );
        assert_eq!(
            scripts
                .stage_codes(Language::Keystone, &"dig-01".into())
                .unwrap(),
            &["dig"]
        );
        let unmapped: LegacyScripts = ron::de::from_bytes(
            &storage
                .load("stage_scripts.legacy-unmapped.ron")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(unmapped.scripts[&Language::Rhai][&LegacyId(99)], ["keep"]);
        let writes = *storage.writes.borrow();
        let again = StageScripts::load_or_default(&storage);
        assert_eq!(again.scripts, scripts.scripts);
        assert_eq!(*storage.writes.borrow(), writes);
    }
    #[test]
    fn failures_preserve_source_and_retry() {
        for failed in [
            STAGE_SCRIPTS_FILE,
            "stage_scripts.legacy-unmapped.ron",
            "stage_scripts.ron.bak",
        ] {
            let storage = MemoryStorage::default();
            let bytes = br#"(scripts:{Rhai:{(1):"code",(99):"keep"}})"#;
            storage.save(STAGE_SCRIPTS_FILE, bytes).unwrap();
            *storage.fail.borrow_mut() = Some(failed.into());
            let mut scripts = StageScripts::load_or_default(&storage);
            scripts.set_stage_codes(Language::Rhai, "move-02".into(), vec!["new".into()]);
            assert!(scripts.persist(&storage).is_err());
            assert_eq!(storage.load(STAGE_SCRIPTS_FILE).unwrap().unwrap(), bytes);
            *storage.fail.borrow_mut() = None;
            let again = StageScripts::load_or_default(&storage);
            assert_eq!(
                again
                    .stage_codes(Language::Rhai, &"move-01".into())
                    .unwrap(),
                &["code"]
            );
        }
    }
    #[test]
    fn corrupt_bytes_get_numbered_backups() {
        let storage = MemoryStorage::default();
        storage.save("stage_scripts.ron.bak", b"older").unwrap();
        storage.save(STAGE_SCRIPTS_FILE, b"corrupt").unwrap();
        StageScripts::load_or_default(&storage)
            .persist(&storage)
            .unwrap();
        assert_eq!(
            storage.load("stage_scripts.ron.bak").unwrap().unwrap(),
            b"older"
        );
        assert_eq!(
            storage.load("stage_scripts.ron.bak.1").unwrap().unwrap(),
            b"corrupt"
        );
    }
}
