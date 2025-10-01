use bevy::prelude::*;
use bevy_egui::{EguiPrimaryContextPass};

use crate::adapter::*;

mod components;
mod systems;

pub struct StagePlugin;
impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(EguiPrimaryContextPass, systems::ui.run_if(in_state(GameState::Stage)));
    }
}

