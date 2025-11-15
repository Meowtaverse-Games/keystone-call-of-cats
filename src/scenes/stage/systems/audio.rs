use bevy::{
    audio::{AudioPlayer, PlaybackSettings},
    prelude::*,
};

pub const STONE_PUSH_SFX_PATH: &str = "audio/stone_push.wav";
pub const STAGE_CLEAR_SFX_PATH: &str = "audio/stage_clear.wav";
const STONE_PUSH_SFX_COOLDOWN: f32 = 0.2;

#[derive(Resource, Clone)]
pub struct StageAudioHandles {
    pub stone_move: Handle<AudioSource>,
    pub stage_clear: Handle<AudioSource>,
}

impl StageAudioHandles {
    pub fn new(stone_move: Handle<AudioSource>, stage_clear: Handle<AudioSource>) -> Self {
        Self {
            stone_move,
            stage_clear,
        }
    }
}

#[derive(Resource)]
pub struct StageAudioState {
    time_since_push: f32,
    clear_played: bool,
}

impl Default for StageAudioState {
    fn default() -> Self {
        Self {
            time_since_push: STONE_PUSH_SFX_COOLDOWN,
            clear_played: false,
        }
    }
}

impl StageAudioState {
    pub fn tick(&mut self, delta: f32) {
        self.time_since_push = (self.time_since_push + delta).min(STONE_PUSH_SFX_COOLDOWN);
    }

    pub fn play_push_if_ready(&mut self, commands: &mut Commands, handles: &StageAudioHandles) {
        if self.time_since_push < STONE_PUSH_SFX_COOLDOWN {
            return;
        }
        commands.spawn((
            AudioPlayer::new(handles.stone_move.clone()),
            PlaybackSettings::DESPAWN,
        ));
        self.time_since_push = 0.0;
    }

    pub fn play_clear_once(&mut self, commands: &mut Commands, handles: &StageAudioHandles) {
        if self.clear_played {
            return;
        }
        commands.spawn((
            AudioPlayer::new(handles.stage_clear.clone()),
            PlaybackSettings::DESPAWN,
        ));
        self.clear_played = true;
    }
}
