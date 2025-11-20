use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    file_storage::{FileError, FileStorage},
    script_engine::Language,
};

pub const GAME_SETTINGS_FILE: &str = "game_settings.ron";

fn clamp_unit(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameSettings {
    master_volume: f32,
    sfx_volume: f32,
    music_volume: f32,
    pub fullscreen: bool,
    pub script_language: Language,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            sfx_volume: 0.9,
            music_volume: 0.7,
            fullscreen: false,
            script_language: Language::Rhai,
        }
    }
}

impl GameSettings {
    pub fn load_or_default(storage: &dyn FileStorage) -> Self {
        match storage.load(GAME_SETTINGS_FILE) {
            Ok(Some(bytes)) => ron::de::from_bytes(&bytes).unwrap_or_else(|err| {
                warn!("Failed to parse settings data: {err}");
                Self::default()
            }),
            Ok(None) => Self::default(),
            Err(err) => {
                warn!("Failed to load settings: {err}");
                Self::default()
            }
        }
    }

    pub fn persist(&self, storage: &dyn FileStorage) -> Result<(), FileError> {
        let serialized = ron::ser::to_string(self)
            .map_err(|err| FileError::Other(format!("serialize settings: {err}")))?;
        storage.save(GAME_SETTINGS_FILE, serialized.as_bytes())
    }

    pub fn master_volume_percent(&self) -> f32 {
        clamp_unit(self.master_volume) * 100.0
    }

    pub fn set_master_volume_percent(&mut self, value: f32) {
        self.master_volume = clamp_unit(value * 0.01);
    }

    pub fn sfx_volume_percent(&self) -> f32 {
        clamp_unit(self.sfx_volume) * 100.0
    }

    pub fn set_sfx_volume_percent(&mut self, value: f32) {
        self.sfx_volume = clamp_unit(value * 0.01);
    }

    pub fn music_volume_percent(&self) -> f32 {
        clamp_unit(self.music_volume) * 100.0
    }

    pub fn set_music_volume_percent(&mut self, value: f32) {
        self.music_volume = clamp_unit(value * 0.01);
    }

    pub fn sfx_volume_linear(&self) -> f32 {
        clamp_unit(self.master_volume) * clamp_unit(self.sfx_volume)
    }

    pub fn music_volume_linear(&self) -> f32 {
        clamp_unit(self.master_volume) * clamp_unit(self.music_volume)
    }
}
