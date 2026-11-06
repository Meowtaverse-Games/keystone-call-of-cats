use bevy::prelude::Resource;
use std::{io, sync::Arc};
use thiserror::Error;

pub mod local;
#[cfg(feature = "steam")]
pub mod steam_cloud;

pub use local::LocalFileStorage;
#[cfg(feature = "steam")]
pub use steam_cloud::SteamCloudFileStorage;

pub trait FileStorage {
    fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError>;
    fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError>;
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("storage is unavailable")]
    #[allow(dead_code)]
    Unavailable,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Resource, Clone)]
pub struct FileStorageResource {
    backend: Arc<dyn FileStorage + Send + Sync>,
}

impl FileStorageResource {
    pub fn new(backend: Arc<dyn FileStorage + Send + Sync>) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> Arc<dyn FileStorage + Send + Sync> {
        self.backend.clone()
    }
}

impl std::ops::Deref for FileStorageResource {
    type Target = dyn FileStorage + Send + Sync;

    fn deref(&self) -> &Self::Target {
        &*self.backend
    }
}

/// Preserve original bytes before replacing legacy or unreadable data. Never overwrite a backup.
pub fn backup(storage: &dyn FileStorage, name: &str, bytes: &[u8]) -> Result<(), FileError> {
    for index in 0usize.. {
        let path = if index == 0 {
            format!("{name}.bak")
        } else {
            format!("{name}.bak.{index}")
        };
        if storage.load(&path)?.is_none() {
            storage.save(&path, bytes)?;
            bevy::log::info!("Preserved {name} ({} bytes) in {path}", bytes.len());
            return Ok(());
        }
    }
    unreachable!()
}

#[cfg(test)]
pub mod test_storage {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};
    #[derive(Default)]
    pub struct MemoryStorage {
        pub files: RefCell<HashMap<String, Vec<u8>>>,
        pub fail: RefCell<Option<String>>,
        pub writes: RefCell<usize>,
        pub fail_load: RefCell<bool>,
    }
    impl FileStorage for MemoryStorage {
        fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError> {
            if *self.fail_load.borrow() {
                return Err(FileError::Unavailable);
            }
            Ok(self.files.borrow().get(name).cloned())
        }
        fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError> {
            if self.fail.borrow().as_deref() == Some(name) {
                return Err(FileError::Unavailable);
            }
            self.files.borrow_mut().insert(name.into(), bytes.to_vec());
            *self.writes.borrow_mut() += 1;
            Ok(())
        }
    }
}
