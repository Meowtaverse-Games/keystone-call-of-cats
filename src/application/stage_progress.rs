use ron::de::from_bytes;
use ron::ser::to_string;
use thiserror::Error;

use crate::{
    application::steam::{RemoteFile, RemoteFileError, RemoteFileStorage},
    domain::stage_progress::StageProgress,
};

const PROGRESS_FILE: RemoteFile = RemoteFile::StageProgress;

pub struct StageProgressUseCase<'a, S: RemoteFileStorage> {
    storage: &'a S,
}

impl<'a, S: RemoteFileStorage> StageProgressUseCase<'a, S> {
    pub fn new(storage: &'a S) -> Self {
        Self { storage }
    }

    pub fn load_or_default(&self) -> Result<StageProgress, StageProgressError> {
        match self.storage.load_remote_file(PROGRESS_FILE)? {
            Some(bytes) => Ok(from_bytes(&bytes)?),
            None => Ok(StageProgress::default()),
        }
    }

    pub fn save(&self, progress: &StageProgress) -> Result<(), StageProgressError> {
        let encoded = to_string(progress)?;
        self.storage
            .save_remote_file(PROGRESS_FILE, encoded.as_bytes())?;
        Ok(())
    }

    pub fn unlock_stage(&self, stage_index: usize) -> Result<StageProgress, StageProgressError> {
        let mut progress = self.load_or_default()?;
        if progress.unlock_until(stage_index) {
            self.save(&progress)?;
        }
        Ok(progress)
    }
}

#[derive(Debug, Error)]
pub enum StageProgressError {
    #[error(transparent)]
    Storage(#[from] RemoteFileError),
    #[error("failed to deserialize stage progress: {0}")]
    Deserialize(#[from] ron::error::SpannedError),
    #[error("failed to serialize stage progress: {0}")]
    Serialize(#[from] ron::error::Error),
}
