use std::{collections::HashMap, sync::Mutex};

use crate::resources::file_storage::{FileError, FileStorage};

/// Ephemeral browser storage used by the wasm build.
///
/// Data is retained while the page is open, but is intentionally not persisted
/// across reloads. Native builds continue to use `LocalFileStorage`.
#[derive(Debug, Default)]
pub struct MemoryFileStorage {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

impl FileStorage for MemoryFileStorage {
    fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError> {
        let files = self
            .files
            .lock()
            .map_err(|_| FileError::Other("browser storage lock was poisoned".to_string()))?;
        Ok(files.get(name).cloned())
    }

    fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError> {
        let mut files = self
            .files
            .lock()
            .map_err(|_| FileError::Other("browser storage lock was poisoned".to_string()))?;
        files.insert(name.to_string(), bytes.to_vec());
        Ok(())
    }
}
