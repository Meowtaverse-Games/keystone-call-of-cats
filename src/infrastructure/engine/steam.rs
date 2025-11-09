use bevy::prelude::*;

use std::io::{Read, Write};
use thiserror::Error;

pub enum RemoteFileType {
    Stages,
}

#[derive(Resource, Default)]
pub struct SteamClient {
    client: Option<bevy_steamworks::Client>,
}

#[derive(Error, Debug)]
pub enum SteamError {
    #[error("Steam client not initialized")]
    SteamNotInitialized,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

fn file_name(file_type: RemoteFileType) -> String {
    match file_type {
        RemoteFileType::Stages => "stages.txt".to_string(),
    }
}
impl SteamClient {
    pub fn save(
        &self,
        file_type: RemoteFileType,
        contents: String,
    ) -> std::result::Result<(), SteamError> {
        let Some(client) = &self.client else {
            return Err(SteamError::SteamNotInitialized);
        };

        let rs = client.remote_storage();

        info!(
            "cloud_enabled_app={} cloud_enabled_user={}",
            rs.is_cloud_enabled_for_app(),
            rs.is_cloud_enabled_for_account()
        );

        let filename = file_name(file_type);
        let file = rs.file(&filename);
        let mut writer = file.write();
        writer.write_all(contents.as_bytes())?;

        Ok(())
    }

    pub fn load(
        &self,
        file_type: RemoteFileType,
    ) -> std::result::Result<Option<String>, SteamError> {
        let Some(client) = &self.client else {
            return Err(SteamError::SteamNotInitialized);
        };

        let rs = client.remote_storage();

        let filename = file_name(file_type);
        let file = rs.file(&filename);

        if !file.exists() {
            return Ok(None);
        }

        let mut reader = file.read();
        let mut contents = String::new();
        reader.read_to_string(&mut contents)?;

        Ok(Some(contents))
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
