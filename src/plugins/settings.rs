use bevy::{
    audio::{PlaybackSettings, Volume},
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};

use crate::{
    resources::{file_storage::FileStorageResource, settings::GameSettings},
    scenes::audio::BackgroundMusic,
};

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSettings>()
            .add_systems(PostStartup, load_settings)
            .add_systems(
                Update,
                (persist_settings, apply_window_settings, update_bgm_volume),
            );
    }
}

fn load_settings(mut commands: Commands, storage: Option<Res<FileStorageResource>>) {
    let Some(storage) = storage else {
        return;
    };
    let loaded = GameSettings::load_or_default(storage.backend().as_ref());
    commands.insert_resource(loaded);
}

fn persist_settings(settings: Res<GameSettings>, storage: Option<Res<FileStorageResource>>) {
    if !settings.is_changed() {
        return;
    }

    let Some(storage) = storage else {
        return;
    };

    if let Err(err) = settings.persist(storage.backend().as_ref()) {
        warn!("Failed to save settings: {err}");
    }
}

fn apply_window_settings(
    settings: Res<GameSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }

    let Ok(mut window) = windows.single_mut() else {
        return;
    };

    let desired = if settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
    } else {
        WindowMode::Windowed
    };

    if window.mode != desired {
        window.mode = desired;
    }
}

fn update_bgm_volume(
    settings: Res<GameSettings>,
    mut players: Query<&mut PlaybackSettings, With<BackgroundMusic>>,
) {
    if !settings.is_changed() {
        return;
    }

    let volume = Volume::Linear(settings.music_volume_linear());
    for mut playback in &mut players {
        playback.volume = volume;
    }
}
