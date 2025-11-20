use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::{
    resources::game_state::GameState,
    scenes::options::{OptionsOverlayState, handle_overlay_input, options_overlay_ui},
};

mod components;
mod systems;

pub struct StageSelectPlugin;

impl Plugin for StageSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OptionsOverlayState>()
            .add_systems(
                OnEnter(GameState::SelectStage),
                (systems::setup, systems::setup_bgm),
            )
            .add_systems(
                Update,
                (
                    handle_overlay_input,
                    systems::handle_back_button,
                    systems::handle_options_button,
                    systems::handle_nav_buttons,
                    systems::handle_play_buttons,
                    systems::handle_keyboard_navigation,
                    systems::refresh_cards,
                    systems::update_page_indicator,
                    systems::update_button_visuals,
                )
                    .run_if(in_state(GameState::SelectStage)),
            )
            .add_systems(OnExit(GameState::SelectStage), systems::cleanup)
            .add_systems(
                EguiPrimaryContextPass,
                options_overlay_ui.run_if(in_state(GameState::SelectStage)),
            );
    }
}
