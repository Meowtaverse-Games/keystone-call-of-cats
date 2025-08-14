use bevy::prelude::*;
use keystone_cc_adapter::*;

mod components;
mod systems;

pub struct TitlePlugin;
impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), systems::setup)


            .add_systems(OnEnter(GameState::Title), systems::draw)

            .add_systems(OnExit(GameState::Title), systems::cleanup);
    }
}
