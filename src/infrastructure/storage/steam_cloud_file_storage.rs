use std::io::{Read, Write};
use steamworks::RemoteStoragePlatforms;

use crate::application::ports::file_storage::{FileError, FileStorage};

/// Steam Remote Storage による FileStorage 実装。
/// 注意: ファイル名は仮想的な相対パスを使用してください（先頭に '/' を付けない）。
#[derive(Clone)]
pub struct SteamCloudFileStorage {
    client: bevy_steamworks::Client,
}

impl SteamCloudFileStorage {
    pub fn new(client: bevy_steamworks::Client) -> Self {
        Self { client }
    }

    fn ensure_cloud_enabled(&self) -> Result<(), FileError> {
        let rs = self.client.remote_storage();
        if rs.is_cloud_enabled_for_app() && rs.is_cloud_enabled_for_account() {
            Ok(())
        } else {
            Err(FileError::Unavailable)
        }
    }

    #[inline]
    fn sanitize_name<'a>(&self, name: &'a str) -> &'a str {
        // Remote Storage は先頭スラッシュ付きの絶対パスを受け付けない。
        name.strip_prefix('/').unwrap_or(name)
    }
}

impl FileStorage for SteamCloudFileStorage {
    fn load(&self, name: &str) -> Result<Option<Vec<u8>>, FileError> {
        self.ensure_cloud_enabled()?;
        let name = self.sanitize_name(name);
        let rs = self.client.remote_storage();
        let file = rs.file(name);
        if !file.exists() {
            return Ok(None);
        }
        let mut reader = file.read();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(Some(buf))
    }

    fn save(&self, name: &str, bytes: &[u8]) -> Result<(), FileError> {
        self.ensure_cloud_enabled()?;
        let name = self.sanitize_name(name);
        let rs = self.client.remote_storage();

        // 任意: 全プラットフォーム同期（必要に応じて）
        rs.file(name).set_sync_platforms(
            RemoteStoragePlatforms::WINDOWS
                | RemoteStoragePlatforms::MACOS
                | RemoteStoragePlatforms::LINUX,
        );

        {
            let mut writer = rs.file(name).write();
            writer.write_all(bytes)?; // DropでCloseされる
        }

        Ok(())
    }
}
