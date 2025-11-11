use crate::application::ports::{StageRepoError, StageRepository};
use crate::domain::stage::{StageId, StageMeta};

/// A simple in-memory implementation of `StageRepository` backed by a static list.
///
/// This adapter belongs to the infrastructure layer: it knows how to *provide* stage meta
/// data but contains no domain logic beyond returning the predefined set.
///
/// Replace or extend this with a file-based / remote implementation later if needed.
pub struct StaticStageRepository {
    stages: Vec<StageMeta>,
}

impl StaticStageRepository {
    /// Create with a contiguous range [0, count) generating titles like `STAGE-<index>`.
    pub fn new(count: usize) -> Self {
        let stages = (0..count)
            .map(|i| StageMeta {
                id: StageId(i),
                title: format!("STAGE-{}", i),
            })
            .collect();
        Self { stages }
    }

    /// Create from arbitrary titles; index position is used as id.
    pub fn from_titles<T: AsRef<str>>(titles: &[T]) -> Self {
        let stages = titles
            .iter()
            .enumerate()
            .map(|(i, t)| StageMeta {
                id: StageId(i),
                title: t.as_ref().to_string(),
            })
            .collect();
        Self { stages }
    }
}

impl StageRepository for StaticStageRepository {
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
    fn list_returns_all() {
        let repo = StaticStageRepository::new(3);
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].title, "STAGE-0");
    }

    #[test]
    fn get_specific() {
        let repo = StaticStageRepository::from_titles(&["One", "Two"]);
        let stage = repo.get(StageId(1)).unwrap().unwrap();
        assert_eq!(stage.title, "Two");
    }
}
