use crate::application::ports::file_storage::FileStorage;
use crate::domain::stage_progress::StageProgress;
use bevy::prelude::Resource;
use std::sync::Arc;

use ron::{de::from_bytes, ser::to_string};

#[derive(Debug)]
#[allow(dead_code)]
pub enum StageProgressUsecaseError {
    StorageUnavailable,
    Io(String),
    Serialize(String),
    Deserialize(String),
}

impl From<ron::error::SpannedError> for StageProgressUsecaseError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::Deserialize(e.to_string())
    }
}
impl From<ron::error::Error> for StageProgressUsecaseError {
    fn from(e: ron::error::Error) -> Self {
        Self::Serialize(e.to_string())
    }
}

/// ファイルストレージを通じて StageProgress をロード/保存するユースケース。
pub struct StageProgressService<'a> {
    storage: &'a dyn FileStorage,
    file_name: &'a str,
}

impl<'a> StageProgressService<'a> {
    pub fn new(storage: &'a dyn FileStorage) -> Self {
        Self {
            storage,
            file_name: "stage_progress.ron",
        }
    }

    pub fn load_or_default(&self) -> Result<StageProgress, StageProgressUsecaseError> {
        match self.storage.load(self.file_name) {
            Ok(Some(bytes)) => Ok(from_bytes(&bytes)?),
            Ok(None) => Ok(StageProgress::default()),
            Err(e) => match e {
                crate::application::ports::file_storage::FileError::Unavailable => {
                    Ok(StageProgress::default())
                }
                crate::application::ports::file_storage::FileError::Io(ioe) => {
                    Err(StageProgressUsecaseError::Io(ioe.to_string()))
                }
            },
        }
    }

    pub fn save(&self, progress: &StageProgress) -> Result<(), StageProgressUsecaseError> {
        let encoded = to_string(progress)?;
        self.storage
            .save(self.file_name, encoded.as_bytes())
            .map_err(|e| match e {
                crate::application::ports::file_storage::FileError::Unavailable => {
                    StageProgressUsecaseError::StorageUnavailable
                }
                crate::application::ports::file_storage::FileError::Io(ioe) => {
                    StageProgressUsecaseError::Io(ioe.to_string())
                }
            })
    }

    pub fn unlock_stage(
        &self,
        stage_index: usize,
    ) -> Result<StageProgress, StageProgressUsecaseError> {
        let mut progress = self.load_or_default()?;
        if progress.unlock_until(stage_index) {
            self.save(&progress)?;
        }
        Ok(progress)
    }
}

/// ECS Resource version holding an owned Arc to the storage.
#[derive(Resource, Clone)]
pub struct StageProgressServiceRes {
    storage: Arc<dyn FileStorage + Send + Sync>,
    file_name: &'static str,
}

impl StageProgressServiceRes {
    pub fn new(storage: Arc<dyn FileStorage + Send + Sync>) -> Self {
        Self {
            storage,
            file_name: "stage_progress.ron",
        }
    }

    pub fn load_or_default(&self) -> Result<StageProgress, StageProgressUsecaseError> {
        match self.storage.load(self.file_name) {
            Ok(Some(bytes)) => Ok(from_bytes(&bytes)?),
            Ok(None) => Ok(StageProgress::default()),
            Err(e) => match e {
                crate::application::ports::file_storage::FileError::Unavailable => {
                    Ok(StageProgress::default())
                }
                crate::application::ports::file_storage::FileError::Io(ioe) => {
                    Err(StageProgressUsecaseError::Io(ioe.to_string()))
                }
            },
        }
    }

    pub fn save(&self, progress: &StageProgress) -> Result<(), StageProgressUsecaseError> {
        let encoded = to_string(progress)?;
        self.storage
            .save(self.file_name, encoded.as_bytes())
            .map_err(|e| match e {
                crate::application::ports::file_storage::FileError::Unavailable => {
                    StageProgressUsecaseError::StorageUnavailable
                }
                crate::application::ports::file_storage::FileError::Io(ioe) => {
                    StageProgressUsecaseError::Io(ioe.to_string())
                }
            })
    }

    pub fn unlock_stage(
        &self,
        stage_index: usize,
    ) -> Result<StageProgress, StageProgressUsecaseError> {
        let mut progress = self.load_or_default()?;
        if progress.unlock_until(stage_index) {
            self.save(&progress)?;
        }
        Ok(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::file_storage::{FileError, FileStorage};

    struct MemFs(std::collections::HashMap<String, Vec<u8>>);
    impl FileStorage for MemFs {
        fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError> {
            Ok(self.0.get(name).cloned())
        }
        fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError> {
            let mut map = self.0.clone();
            map.insert(name.to_string(), bytes.to_vec());
            // No mutation of original, but this is a minimal placeholder test implementation
            Ok(())
        }
    }

    #[test]
    fn default_load_returns_progress() {
        let fs = MemFs(Default::default());
        let svc = StageProgressService::new(&fs);
        let p = svc.load_or_default().unwrap();
        assert!(p.unlocked_slots() >= 1);
    }
}
