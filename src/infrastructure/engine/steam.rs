use bevy::prelude::*;

use std::io::{Read, Write};

use crate::application::steam::{RemoteFile, RemoteFileError, RemoteFileStorage};

#[derive(Resource, Default)]
pub struct SteamClient {
    client: Option<bevy_steamworks::Client>,
}

impl SteamClient {
    fn remote_storage(&self) -> Result<bevy_steamworks::RemoteStorage, RemoteFileError> {
        let client = self.client.as_ref().ok_or(RemoteFileError::Unavailable)?;
        Ok(client.remote_storage())
    }
}

impl RemoteFileStorage for SteamClient {
    fn load_remote_file(&self, file: RemoteFile) -> Result<Option<Vec<u8>>, RemoteFileError> {
        let storage = self.remote_storage()?;
        let filename = file_name(file);
        let file = storage.file(filename);

        if !file.exists() {
            return Ok(None);
        }

        let mut reader = file.read();
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        Ok(Some(buffer))
    }

    fn save_remote_file(&self, file: RemoteFile, bytes: &[u8]) -> Result<(), RemoteFileError> {
        let storage = self.remote_storage()?;
        let filename = file_name(file);
        let file = storage.file(filename);
        let mut writer = file.write();
        writer.write_all(bytes)?;
        Ok(())
    }
}

fn file_name(file_type: RemoteFile) -> &'static str {
    match file_type {
        RemoteFile::StageProgress => "stage_progress.ron",
    }
}

pub struct SteamPlugin {
    pub app_id: u32,
}

impl SteamPlugin {
    pub fn new(app_id: u32) -> Self {
        Self { app_id }
    }
}

impl Plugin for SteamPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_steamworks::SteamworksPlugin::init_app(self.app_id).unwrap())
            .insert_resource(SteamClient { client: None })
            .add_systems(Startup, setup_steam);
    }
}

fn setup_steam(mut steam: ResMut<SteamClient>, client: Res<bevy_steamworks::Client>) {
    steam.client = Some(client.clone());

    info! {"Steam Cloud Enabled: {}", client.remote_storage().is_cloud_enabled_for_app()};
}
