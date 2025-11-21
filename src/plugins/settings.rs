use bevy::{
    audio::{PlaybackSettings, Volume},
    ecs::system::ParamSet,
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};

use crate::{
    resources::{file_storage::FileStorageResource, settings::GameSettings},
    scenes::audio::*,
};

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSettings>()
            .add_systems(PostStartup, load_settings)
            .add_systems(
                Update,
                (
                    persist_settings,
                    apply_window_settings,
                    update_audio_volumes,
                ),
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

fn update_audio_volumes(
    settings: Res<GameSettings>,
    mut audio_queries: ParamSet<(
        Query<&mut PlaybackSettings, With<BackgroundMusic>>,
        Query<&mut PlaybackSettings, With<SfxAudio>>,
        Query<&mut PlaybackSettings, With<LoopingAudio>>,
    )>,
) {
    if !settings.is_changed() {
        return;
    }
    info!("Updating audio volumes due to settings change");

    // Update background music volume
    let music_volume = Volume::Linear(settings.music_volume_linear());
    for mut playback in &mut audio_queries.p0() {
        if playback.volume != music_volume {
            playback.volume = music_volume;
        }
    }

    // Update SFX volume
    let sfx_volume = Volume::Linear(settings.sfx_volume_linear());
    for mut playback in &mut audio_queries.p1() {
        if playback.volume != sfx_volume {
            playback.volume = sfx_volume;
        }
    }

    // Update looping audio volume (stage effects, etc.)
    for mut playback in &mut audio_queries.p2() {
        if playback.volume != sfx_volume {
            playback.volume = sfx_volume;
        }
    }
}
