use bevy::prelude::*;

use crate::application::*;

mod components;
mod systems;

pub struct StageSelectPlugin;

impl Plugin for StageSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::SelectStage), systems::setup)
            .add_systems(
                Update,
                (
                    systems::handle_back_button,
                    systems::handle_nav_buttons,
                    systems::handle_play_buttons,
                    systems::handle_keyboard_navigation,
                    systems::refresh_cards,
                    systems::update_page_indicator,
                    systems::update_button_visuals,
                )
                    .run_if(in_state(GameState::SelectStage)),
            )
            .add_systems(OnExit(GameState::SelectStage), systems::cleanup);
    }
}
