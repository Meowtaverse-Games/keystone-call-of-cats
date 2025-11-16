use bevy::{
    audio::{AudioPlayer, PlaybackSettings},
    prelude::*,
};

pub const STONE_PUSH_SFX_PATH: &str = "audio/stone_push.wav";
pub const STAGE_CLEAR_SFX_PATH: &str = "audio/stage_clear.wav";

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

#[derive(Resource, Default)]
pub struct StageAudioState {
    clear_played: bool,
    push_loop_entity: Option<Entity>,
}

impl StageAudioState {
    pub fn reset(&mut self, commands: &mut Commands) {
        self.clear_played = false;
        self.stop_push_loop(commands);
    }

    pub fn ensure_push_loop(&mut self, commands: &mut Commands, handles: &StageAudioHandles) {
        if self.push_loop_entity.is_some() {
            return;
        }
        let entity = commands
            .spawn((
                AudioPlayer::new(handles.stone_move.clone()),
                PlaybackSettings::LOOP,
            ))
            .id();
        self.push_loop_entity = Some(entity);
    }

    pub fn stop_push_loop(&mut self, commands: &mut Commands) {
        if let Some(entity) = self.push_loop_entity.take()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
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
