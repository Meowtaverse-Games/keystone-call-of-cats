use bevy::prelude::*;

use crate::adapter::*;

mod components;
mod systems;

pub struct BootPlugin;
impl Plugin for BootPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .add_systems(OnEnter(GameState::Boot), systems::setup)
            .add_systems(Update, systems::update.run_if(in_state(GameState::Boot)))
            .add_systems(OnExit(GameState::Boot), systems::cleanup);
    }
}
