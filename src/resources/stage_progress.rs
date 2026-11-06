use crate::resources::{
    file_storage::{FileError, FileStorage, backup},
    legacy_stage_ids::{LEGACY_STAGE_IDS, legacy_stage_id},
    stage_catalog::{StageCatalog, StageId},
};
use bevy::prelude::{Resource, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const STAGE_PROGRESS_FILE: &str = "stage_progress.ron";
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageProgress {
    pub cleared: HashSet<StageId>,
    pub last_played_stage_id: Option<StageId>,
    #[serde(skip)]
    write_blocked: bool,
}
#[derive(Deserialize)]
struct LegacyId(usize);
#[derive(Deserialize)]
struct LegacyProgress {
    unlocked_until: LegacyId,
    #[serde(default)]
    last_played_stage_id: Option<LegacyId>,
}
impl StageProgress {
    pub fn is_unlocked(&self, catalog: &StageCatalog, id: &StageId) -> bool {
        catalog.is_unlocked(id, &self.cleared)
    }
    pub fn clear(&mut self, id: StageId) {
        self.cleared.insert(id);
    }
    pub fn set_last_played(&mut self, id: StageId, storage: &dyn FileStorage) {
        if self.last_played_stage_id.as_ref() != Some(&id) {
            self.last_played_stage_id = Some(id);
            if let Err(err) = self.persist(storage) {
                warn!("Failed to save last stage: {err}");
            }
        }
    }
    pub fn load_or_default(_catalog: &StageCatalog, storage: &dyn FileStorage) -> Self {
        let bytes = match storage.load(STAGE_PROGRESS_FILE) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Self::default(),
            Err(err) => {
                warn!("Failed to load stage progress; writes disabled: {err}");
                return Self {
                    write_blocked: true,
                    ..Self::default()
                };
            }
        };
        if let Ok(current) = ron::de::from_bytes::<Self>(&bytes) {
            return current;
        }
        let preserved = backup(storage, STAGE_PROGRESS_FILE, &bytes);
        if let Err(err) = &preserved {
            warn!("Cannot preserve stage progress; writes disabled: {err}");
        }
        if let Ok(old) = ron::de::from_bytes::<LegacyProgress>(&bytes) {
            let n = old.unlocked_until.0;
            let count = if n >= 12 { 12 } else { n.saturating_sub(1) };
            let mut progress = Self {
                cleared: LEGACY_STAGE_IDS
                    .iter()
                    .take(count)
                    .map(|id| StageId::from(*id))
                    .collect(),
                last_played_stage_id: old.last_played_stage_id.and_then(|id| {
                    let mapped = legacy_stage_id(id.0).map(StageId::from);
                    if mapped.is_none() {
                        warn!("Discarding unknown last played stage {}", id.0);
                        info!("Progress migration: unmapped last played number {}", id.0);
                    }
                    mapped
                }),
                write_blocked: preserved.is_err(),
            };
            if let Err(err) = progress.persist(storage) {
                warn!("Progress migration save failed; retry on next launch: {err}");
                progress.write_blocked = true;
            } else {
                info!("Migrated stage progress: {count} cleared stages from unlocked_until {n}");
            }
            progress
        } else {
            warn!("Unreadable stage progress; using defaults after backup attempt");
            Self {
                write_blocked: preserved.is_err(),
                ..Self::default()
            }
        }
    }
    pub fn persist(&self, storage: &dyn FileStorage) -> Result<(), FileError> {
        if self.write_blocked {
            return Err(FileError::Other(
                "stage progress writes blocked to preserve original data".into(),
            ));
        }
        let bytes = ron::ser::to_string(self).map_err(|e| FileError::Other(e.to_string()))?;
        storage.save(STAGE_PROGRESS_FILE, bytes.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::file_storage::test_storage::MemoryStorage;
    #[test]
    fn failed_reads_or_backups_never_allow_default_overwrites() {
        let catalog = StageCatalog::load_from_assets();
        let storage = MemoryStorage::default();
        storage.save(STAGE_PROGRESS_FILE, b"corrupt").unwrap();
        *storage.fail.borrow_mut() = Some("stage_progress.ron.bak".into());
        let progress = StageProgress::load_or_default(&catalog, &storage);
        *storage.fail.borrow_mut() = None;
        assert!(progress.persist(&storage).is_err());
        *storage.fail_load.borrow_mut() = true;
        let progress = StageProgress::load_or_default(&catalog, &storage);
        *storage.fail_load.borrow_mut() = false;
        assert!(progress.persist(&storage).is_err());
        assert_eq!(
            storage.load(STAGE_PROGRESS_FILE).unwrap().unwrap(),
            b"corrupt"
        );
    }
    #[test]
    fn migrates_boundaries_last_played_and_only_once() {
        let catalog = StageCatalog::load_from_assets();
        for (n, count) in [(0, 0), (1, 0), (5, 4), (12, 12), (13, 12), (99, 12)] {
            for last in [5, 99] {
                let storage = MemoryStorage::default();
                storage
                    .save(
                        STAGE_PROGRESS_FILE,
                        format!("(unlocked_until:({n}),last_played_stage_id:Some(({last})))")
                            .as_bytes(),
                    )
                    .unwrap();
                let p = StageProgress::load_or_default(&catalog, &storage);
                assert_eq!(p.cleared.len(), count);
                assert_eq!(
                    p.last_played_stage_id,
                    if last == 5 {
                        Some("move-05".into())
                    } else {
                        None
                    }
                );
                for (index, stage) in catalog.iter().enumerate() {
                    assert_eq!(
                        p.is_unlocked(&catalog, &stage.id),
                        stage.unlocked || index <= count
                    );
                }
                let writes = *storage.writes.borrow();
                let again = StageProgress::load_or_default(&catalog, &storage);
                assert_eq!(again.cleared, p.cleared);
                assert_eq!(*storage.writes.borrow(), writes);
            }
        }
    }
    #[test]
    fn preserves_corruption_and_retries_failed_migrations() {
        let catalog = StageCatalog::load_from_assets();
        let storage = MemoryStorage::default();
        for bytes in [b"broken".as_slice(), b"(unlocked_until:(5))"] {
            storage.save(STAGE_PROGRESS_FILE, bytes).unwrap();
            *storage.fail.borrow_mut() = Some(STAGE_PROGRESS_FILE.into());
            let p = StageProgress::load_or_default(&catalog, &storage);
            assert!(p.persist(&storage).is_err());
            assert_eq!(storage.load(STAGE_PROGRESS_FILE).unwrap().unwrap(), bytes);
            *storage.fail.borrow_mut() = None;
            let p = StageProgress::load_or_default(&catalog, &storage);
            p.persist(&storage).unwrap();
            assert!(
                storage
                    .files
                    .borrow()
                    .iter()
                    .any(|(name, b)| name.starts_with("stage_progress.ron.bak") && b == bytes)
            );
            if bytes != b"broken" {
                assert_eq!(p.cleared.len(), 4);
            }
        }
    }
}
