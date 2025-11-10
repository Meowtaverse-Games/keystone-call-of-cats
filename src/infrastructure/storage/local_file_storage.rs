use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::application::ports::file_storage::{FileError, FileStorage};

/// OS標準のアプリデータフォルダ配下に保存するローカル FileStorage。
#[derive(Clone, Debug)]
pub struct LocalFileStorage {
    base_dir: PathBuf,
}

impl LocalFileStorage {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// 既定の保存先ディレクトリを返す（OS別）
    pub fn default_base_dir() -> PathBuf {
        // macOS
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join("Library/Application Support/KeystoneCC");
            }
        }
        // Linux
        #[cfg(target_os = "linux")]
        {
            if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
                return PathBuf::from(xdg).join("KeystoneCC");
            }
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(".local/share/KeystoneCC");
            }
        }
        // Windows
        #[cfg(target_os = "windows")]
        {
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                return PathBuf::from(local).join("KeystoneCC");
            }
            if let Some(roam) = std::env::var_os("APPDATA") {
                return PathBuf::from(roam).join("KeystoneCC");
            }
        }
        // Fallback: カレント配下
        PathBuf::from("./KeystoneCC")
    }

    pub fn default_dir() -> Self {
        Self::new(Self::default_base_dir())
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }
}

impl FileStorage for LocalFileStorage {
    fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError> {
        let path = self.path_for(name);
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(FileError::Io(e)),
        }
    }

    fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError> {
        let path = self.path_for(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(FileError::Io)?;
        }
        // 簡易的なアトミック書き込み
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes).map_err(FileError::Io)?;
        fs::rename(&tmp, &path).map_err(FileError::Io)?;
        Ok(())
    }
}
