use thiserror::Error;

use crate::domain::stage::{StageId, StageMeta};

#[derive(Debug, Error)]
pub enum StageRepoError {
    #[error("repository unavailable")]
    Unavailable,
    #[error("{0}")]
    Other(String),
}

pub trait StageRepository: Send + Sync + 'static {
    fn list(&self) -> Result<Vec<StageMeta>, StageRepoError>;
    fn get(&self, id: StageId) -> Result<Option<StageMeta>, StageRepoError>;
}
