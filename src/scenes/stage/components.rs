use bevy::prelude::*;

#[derive(Component)]
pub struct StageUI;

#[derive(Component)]
pub struct StageBackground;

#[derive(Component, Clone, Copy)]
pub struct StageTile {
    pub coord: UVec2,
}

#[derive(Component)]
pub struct Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAnimationState {
    Idle,
    Run,
}

#[derive(Default)]
pub struct PlayerAnimationClips {
    pub idle: Vec<Handle<Image>>,
    pub run: Vec<Handle<Image>>,
}

impl PlayerAnimationClips {
    pub fn frames(&self, state: PlayerAnimationState) -> &[Handle<Image>] {
        match state {
            PlayerAnimationState::Idle => &self.idle,
            PlayerAnimationState::Run => &self.run,
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
    pub min_x: f32,
    pub max_x: f32,
    pub is_moving: bool,
    pub vertical_velocity: f32,
    pub gravity: f32,
    pub jump_speed: f32,
    pub ground_y: f32,
    pub is_jumping: bool,
}
