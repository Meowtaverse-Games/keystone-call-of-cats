use bevy::{
    audio::{AudioPlayer, PlaybackSettings},
    prelude::*,
};

pub const UI_CLICK_SFX_PATH: &str = "audio/ui_click.wav";

#[derive(Resource, Clone)]
pub struct UiAudioHandles {
    pub click: Handle<AudioSource>,
}

pub struct UiAudioPlugin;

impl Plugin for UiAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_ui_audio);
    }
}

fn load_ui_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handles = UiAudioHandles {
        click: asset_server.load(UI_CLICK_SFX_PATH),
    };
    commands.insert_resource(handles);
}

pub fn play_ui_click(commands: &mut Commands, handles: &UiAudioHandles) {
    commands.spawn((
        AudioPlayer::new(handles.click.clone()),
        PlaybackSettings::DESPAWN,
    ));
}
