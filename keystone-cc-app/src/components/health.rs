use bevy::prelude::*;

#[derive(Component)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}
