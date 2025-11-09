use bevy::prelude::*;

use crate::application::*;

mod components;
mod systems;

pub struct TitlePlugin;
impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), systems::setup)
            .add_systems(OnExit(GameState::Title), systems::cleanup);
    }
}
