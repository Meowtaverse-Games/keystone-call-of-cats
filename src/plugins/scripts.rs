use bevy::prelude::*;

use crate::resources::script_engine::ScriptExecutor;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScriptExecutor::default());
    }
}
