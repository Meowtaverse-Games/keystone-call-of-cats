use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::adapter::*;

mod components;
mod systems;

fn ui_example_system(mut contexts: EguiContexts) -> Result {
    egui::Window::new("Hello").show(contexts.ctx_mut()?, |ui| {
        ui.label("world");
    });
    egui::Window::new("Hello").show(contexts.ctx_mut()?, |ui| {
        ui.label("world");
    });
    Ok(())
}

pub struct StagePlugin;
impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Stage), ui_example_system)
            //.add_systems(OnEnter(GameState::State), systems::setup)
            .add_systems(OnExit(GameState::Stage), systems::cleanup);
    }
}

