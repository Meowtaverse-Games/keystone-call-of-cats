use std::fs;
use std::path::{Path, PathBuf};

use crate::application::ports::{StageRepoError, StageRepository};
use crate::domain::stage::{StageId, StageMeta};
use serde::Deserialize;

/// File format for RON stage list: a top-level sequence of entries.
/// Each entry includes at minimum a title; index is implicit by order unless overridden.
#[derive(Debug, Deserialize)]
struct RonStageEntry {
    title: String,
    #[serde(default)]
    id: Option<usize>,
}

/// Repository that loads stage metadata from a RON file.
///
/// Expected RON structure example:
/// [
///   (title: "INTRO"),
///   (title: "BASICS"),
///   (title: "ADVANCED", id: Some(10)), // explicit id override (Option)
/// ]
///
/// If `id` is omitted, the ordering index (0-based) is used.
pub struct FileStageRepository {
    path: PathBuf,
    stages: Vec<StageMeta>,
}

impl FileStageRepository {
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self, StageRepoError> {
        let path_buf = path.as_ref().to_path_buf();
        let content = fs::read_to_string(&path_buf)
            .map_err(|e| StageRepoError::Other(format!("read error: {}", e)))?;
        let entries: Vec<RonStageEntry> = ron::de::from_str(&content)
            .map_err(|e| StageRepoError::Other(format!("RON parse error: {}", e)))?;

        let mut stages = Vec::with_capacity(entries.len());
        for (i, entry) in entries.into_iter().enumerate() {
            let id_val = entry.id.unwrap_or(i);
            stages.push(StageMeta {
                id: StageId(id_val),
                title: entry.title,
            });
        }
        Ok(Self {
            path: path_buf,
            stages,
        })
    }

    pub fn reload(&mut self) -> Result<(), StageRepoError> {
        *self = Self::load_from(&self.path)?;
        Ok(())
    }
}

impl StageRepository for FileStageRepository {
    fn list(&self) -> Result<Vec<StageMeta>, StageRepoError> {
        Ok(self.stages.clone())
    }

    fn get(&self, id: StageId) -> Result<Option<StageMeta>, StageRepoError> {
        Ok(self.stages.iter().find(|m| m.id == id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_inline_ron() {
        let content = "[(title: \"A\"),(title: \"B\"),(title: \"C\")]";
        let entries: Vec<super::RonStageEntry> = ron::de::from_str(content).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].title, "B");
    }

    #[test]
    fn build_stage_meta_from_entries() {
        let content = "[(title: \"X\", id: Some(5)),(title: \"Y\"),(title: \"Z\", id: Some(9))]";
        let entries: Vec<super::RonStageEntry> = ron::de::from_str(content).unwrap();
        let mut metas = Vec::new();
        for (i, e) in entries.into_iter().enumerate() {
            let id_val = e.id.unwrap_or(i);
            metas.push(StageMeta {
                id: StageId(id_val),
                title: e.title,
            });
        }
        assert_eq!(metas[0].id.0, 5);
        assert_eq!(metas[1].id.0, 1);
        assert_eq!(metas[2].id.0, 9);
    }
}
