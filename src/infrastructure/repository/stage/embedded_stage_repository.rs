use crate::application::ports::{StageRepoError, StageRepository};
use crate::domain::stage::{StageId, StageMeta};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RonStageEntry {
    title: String,
    #[serde(default)]
    id: Option<usize>,
}

/// Repository that reads stage metadata from an embedded RON string at compile time.
/// The RON is embedded using include_str! so it doesn't require runtime file I/O.
pub struct EmbeddedStageRepository {
    stages: Vec<StageMeta>,
}

impl EmbeddedStageRepository {
    pub fn load() -> Result<Self, StageRepoError> {
        // Embed the catalog from the assets folder at build time.
        // Using concat!(env!("CARGO_MANIFEST_DIR"), ...) keeps the path stable.
        const CATALOG: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/stages/catalog.ron"
        ));

        let entries: Vec<RonStageEntry> = ron::de::from_str(CATALOG)
            .map_err(|e| StageRepoError::Other(format!("RON parse error: {}", e)))?;
        let mut stages = Vec::with_capacity(entries.len());
        for (i, e) in entries.into_iter().enumerate() {
            let id_val = e.id.unwrap_or(i);
            stages.push(StageMeta {
                id: StageId(id_val),
                title: e.title,
            });
        }
        Ok(Self { stages })
    }
}

impl StageRepository for EmbeddedStageRepository {
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
    fn loads_embedded_catalog() {
        let repo = EmbeddedStageRepository::load().unwrap();
        let list = repo.list().unwrap();
        assert!(!list.is_empty());
    }
}
