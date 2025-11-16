use bevy::{
    audio::{AudioPlayer, PlaybackSettings, Volume},
    prelude::*,
};

const UI_CLICK_SFX_PATH: &str = "audio/ui_click.ogg";
const BGM_PATH: &str = "audio/bgm.wav";

#[derive(Resource, Clone, Default)]
pub struct UiAudioHandles {
    pub click: Handle<AudioSource>,
    pub bgm: Handle<AudioSource>,
    played_bgm: bool,
}

pub struct UIAudioPlugin;

impl Plugin for UIAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_ui_audio);
    }
}

fn load_ui_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handles = UiAudioHandles {
        click: asset_server.load(UI_CLICK_SFX_PATH),
        bgm: asset_server.load(BGM_PATH),
        ..default()
    };
    commands.insert_resource(handles);
}

pub fn play_bgm(commands: &mut Commands, handles: &mut UiAudioHandles) {
    if handles.played_bgm {
        return;
    }

    handles.played_bgm = true;
    info!("Playing BGM");

    commands.spawn((
        AudioPlayer::new(handles.bgm.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.1)),
    ));
}

pub fn play_ui_click(commands: &mut Commands, handles: &UiAudioHandles) {
    commands.spawn((
        AudioPlayer::new(handles.click.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.1)),
    ));
}
