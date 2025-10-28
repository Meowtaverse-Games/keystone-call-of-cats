use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::adapter::*;

mod components;
mod systems;

pub struct StagePlugin;
impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<systems::StoneCommandMessage>()
            .add_systems(OnEnter(GameState::Stage), systems::setup)
            .add_systems(OnExit(GameState::Stage), systems::cleanup)
            .add_systems(
                Update,
                (
                    systems::update_stage_root,
                    systems::animate_character,
                    systems::move_character,
                    systems::handle_stone_messages,
                    systems::update_stone_behavior,
                )
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
