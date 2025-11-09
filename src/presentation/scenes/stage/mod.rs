use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::application::*;

pub mod components;
pub mod systems;

pub use systems::StageProgression;

pub struct StagePlugin;
impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::StageProgression>()
            .add_message::<systems::StoneCommandMessage>()
            .add_systems(OnEnter(GameState::Stage), systems::setup)
            .add_systems(OnExit(GameState::Stage), systems::cleanup)
            .add_systems(
                Update,
                (
                    systems::update_stage_root,
                    systems::reset_player_position,
                    systems::animate_character,
                    systems::move_character,
                    systems::handle_stone_messages,
                    systems::update_stone_behavior,
                )
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::check_goal_completion
                    .after(systems::move_character)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::advance_stage_if_cleared
                    .after(systems::check_goal_completion)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::reload_stage_if_needed
                    .after(systems::advance_stage_if_cleared)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::carry_riders_with_stone
                    .after(systems::update_stone_behavior)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                systems::ui.run_if(in_state(GameState::Stage)),
            );
    }
}
