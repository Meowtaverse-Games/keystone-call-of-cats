use bevy::prelude::*;

use crate::{
    resources::game_state::GameState,
    systems::stage::{
        load::setup_stage_resources,
        progress::persist_stage_progress,
        scripts::{persist_stage_scripts, persist_stage_scripts_on_app_exit},
    },
};

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_stage_resources)
            .add_systems(Update, persist_stage_progress)
            .add_systems(OnExit(GameState::Stage), persist_stage_scripts)
            .add_systems(OnExit(GameState::SelectStage), persist_stage_scripts)
            .add_systems(Last, persist_stage_scripts_on_app_exit);
    }
}
