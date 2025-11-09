use std::io;

use thiserror::Error;

pub const STEAM_APP_ID: u32 = 4169380;

#[derive(Debug, Clone, Copy)]
pub enum RemoteFile {
    StageProgress,
}

#[derive(Debug, Error)]
pub enum RemoteFileError {
    #[error("remote storage is unavailable")]
    Unavailable,
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub trait RemoteFileStorage {
    fn load_remote_file(&self, file: RemoteFile) -> Result<Option<Vec<u8>>, RemoteFileError>;
    fn save_remote_file(&self, file: RemoteFile, bytes: &[u8]) -> Result<(), RemoteFileError>;
}
