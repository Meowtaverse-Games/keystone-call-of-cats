use bevy::prelude::*;

#[derive(Resource)]
pub struct ScriptExecutor;

impl ScriptExecutor {
    pub fn execute_script(&self, language: Language, script: &str) {
        match language {
            Language::Rhai => {
                // Execute Rhai script
            }
            Language::Keystone => {
                // Execute Keystone script
            }
        }
    }
}

pub enum Language {
    Rhai,
    Keystone,
}

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScriptExecutor);
    }
}
