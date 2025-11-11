use bevy::prelude::*;

use crate::systems::stage::{load::setup_stage_resources, progress::persist_stage_progress};

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_stage_resources)
            .add_systems(Update, persist_stage_progress);
    }
}
