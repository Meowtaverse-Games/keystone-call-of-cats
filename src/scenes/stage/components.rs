use bevy::prelude::*;

#[derive(Component)]
pub struct StageUI;

#[derive(Component)]
pub struct StageBackground;

#[derive(Component)]
pub struct StageCharacter;

#[derive(Component)]
pub struct CharacterAnimation {
    pub timer: Timer,
    pub frames: usize,
}

#[derive(Component)]
pub struct CharacterMotion {
    pub speed: f32,
    pub direction: f32,
    pub min_x: f32,
    pub max_x: f32,
}
