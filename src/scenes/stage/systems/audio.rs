use bevy::{
    audio::{AudioPlayer, PlaybackSettings, Volume},
    prelude::*,
};

use crate::scenes::audio::{SfxAudio, LoopingAudio};

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

    pub fn ensure_push_loop(
        &mut self,
        commands: &mut Commands,
        handles: &StageAudioHandles,
        volume: f32,
    ) {
        if self.push_loop_entity.is_some() {
            return;
        }
        let entity = commands
            .spawn((
                LoopingAudio,
                AudioPlayer::new(handles.stone_move.clone()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(volume)),
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

    pub fn play_clear_once(
        &mut self,
        commands: &mut Commands,
        handles: &StageAudioHandles,
        volume: f32,
    ) {
        if self.clear_played {
            return;
        }
        commands.spawn((
            SfxAudio,
            AudioPlayer::new(handles.stage_clear.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
        ));
        self.clear_played = true;
    }
}
