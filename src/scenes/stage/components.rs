use bevy::prelude::*;

#[derive(Component)]
pub struct StageRoot;

#[derive(Component)]
pub struct StageBackground;

#[derive(Component)]
pub struct StageDebugMarker;

#[derive(Component, Clone, Copy)]
pub struct StageTile;

#[derive(Component)]
pub struct StoneRune;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Goal {
    pub half_extents: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAnimationState {
    Idle,
    Run,
    Climb,
}

#[derive(Default)]
pub struct PlayerAnimationClips {
    pub idle: Vec<Handle<Image>>,
    pub run: Vec<Handle<Image>>,
    pub climb: Vec<Handle<Image>>,
}

impl PlayerAnimationClips {
    pub fn frames(&self, state: PlayerAnimationState) -> &[Handle<Image>] {
        match state {
            PlayerAnimationState::Idle => &self.idle,
            PlayerAnimationState::Run => &self.run,
            PlayerAnimationState::Climb => &self.climb,
        }
    }
}

#[derive(Component)]
pub struct PlayerAnimation {
    pub timer: Timer,
    pub clips: PlayerAnimationClips,
    pub state: PlayerAnimationState,
    pub frame_index: usize,
}

impl PlayerAnimation {
    pub fn current_frames(&self) -> &[Handle<Image>] {
        self.clips.frames(self.state)
    }
}

#[derive(Component)]
pub struct PlayerMotion {
    pub speed: f32,
    pub direction: f32,
    pub is_moving: bool,
    pub jump_speed: f32,
    pub ground_y: f32,
    pub is_jumping: bool,
}

#[derive(Component)]
pub struct PlayerSpawnState {
    pub translation: Vec3,
    pub scale: f32,
}
