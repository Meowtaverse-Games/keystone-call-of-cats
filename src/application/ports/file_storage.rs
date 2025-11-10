use std::io;
use thiserror::Error;

pub trait FileStorage {
    fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError>;
    fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError>;
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("storage is unavailable")]
    Unavailable,
    #[error(transparent)]
    Io(#[from] io::Error),
}
