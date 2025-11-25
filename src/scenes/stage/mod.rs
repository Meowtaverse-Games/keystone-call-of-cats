use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::resources::game_state::GameState;

pub mod components;
pub mod systems;

pub use systems::StageProgressionState;

pub struct StageScenePlugin;
impl Plugin for StageScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::StageProgressionState>()
            .add_message::<systems::StoneCommandMessage>()
            .add_message::<systems::StoneAppendCommandMessage>()
            .add_systems(OnEnter(GameState::Stage), systems::setup)
            .add_systems(
                OnEnter(GameState::Stage),
                crate::systems::engine::friction::apply_zero_friction_to_rigid_bodies
                    .after(systems::setup),
            )
            .add_systems(OnExit(GameState::Stage), systems::cleanup)
            .add_systems(
                Update,
                (
                    systems::tick_script_program,
                    systems::update_stage_root,
                    systems::update_stage_color_grading,
                    systems::reset_stone_position,
                    systems::reset_player_position,
                    systems::animate_player,
                    systems::move_player,
                    systems::handle_stone_messages,
                    systems::handle_stone_append_messages,
                    systems::update_stone_behavior,
                )
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::check_goal_completion
                    .after(systems::move_player)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::drive_player_goal_descent
                    .after(systems::check_goal_completion)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::advance_stage_if_cleared
                    .after(systems::drive_player_goal_descent)
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
                Update,
                systems::handle_tutorial_overlay_input.run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                systems::ui.run_if(in_state(GameState::Stage)),
            );
    }
}
