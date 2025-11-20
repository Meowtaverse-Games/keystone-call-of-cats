use bevy::{
    audio::{AudioPlayer, PlaybackSettings, Volume},
    prelude::*,
};

use crate::{
    resources::{asset_store::AssetStore, settings::GameSettings},
    scenes::assets::AudioKey,
};

#[derive(Component)]
pub struct BackgroundMusic;

#[derive(Resource, Clone, Default)]
pub struct AudioHandles {
    pub click: Handle<AudioSource>,
    pub bgm: Handle<AudioSource>,
    played_bgm: bool,
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, init_audio_handles);
    }
}

fn init_audio_handles(
    mut commands: Commands,
    asset_store: Option<Res<AssetStore>>,
    existing: Option<Res<AudioHandles>>,
) {
    if existing.is_some() {
        return;
    }

    let Some(store) = asset_store else {
        return;
    };

    let Some(click) = store.audio(AudioKey::UiClick) else {
        return;
    };
    let Some(bgm) = store.audio(AudioKey::Bgm) else {
        return;
    };

    commands.insert_resource(AudioHandles {
        click,
        bgm,
        ..default()
    });
}

pub fn play_bgm(commands: &mut Commands, handles: &mut AudioHandles, settings: &GameSettings) {
    if handles.played_bgm {
        return;
    }

    handles.played_bgm = true;
    info!("Playing BGM");

    commands.spawn((
        BackgroundMusic,
        AudioPlayer::new(handles.bgm.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.music_volume_linear())),
    ));
}

pub fn play_ui_click(commands: &mut Commands, handles: &AudioHandles, settings: &GameSettings) {
    commands.spawn((
        AudioPlayer::new(handles.click.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(settings.sfx_volume_linear())),
    ));
}
