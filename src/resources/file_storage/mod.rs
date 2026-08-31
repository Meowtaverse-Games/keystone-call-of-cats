use bevy::prelude::Resource;
use std::{io, sync::Arc};
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
pub mod local;
#[cfg(target_arch = "wasm32")]
pub mod memory;
#[cfg(feature = "steam")]
pub mod steam_cloud;

#[cfg(not(target_arch = "wasm32"))]
pub use local::LocalFileStorage;
#[cfg(target_arch = "wasm32")]
pub use memory::MemoryFileStorage;
#[cfg(feature = "steam")]
pub use steam_cloud::SteamCloudFileStorage;

pub trait FileStorage {
    fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError>;
    fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError>;
}

/// Returns the storage backend appropriate for the current platform.
///
/// Browsers cannot access the native application-data directory. The web build
/// therefore keeps save data in memory for the lifetime of the page.
pub fn default_storage() -> Arc<dyn FileStorage + Send + Sync> {
    #[cfg(target_arch = "wasm32")]
    {
        Arc::new(MemoryFileStorage::default())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Arc::new(LocalFileStorage::default_dir())
    }
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
