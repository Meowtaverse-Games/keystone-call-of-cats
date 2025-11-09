use crate::{
    domain::scripts::{ScriptCommand, ScriptExecutionError, ScriptRunner},
    infrastructure::scripts::RhaiScriptExecutor,
};
use bevy::prelude::*;

pub enum Language {
    Rhai,
    #[allow(dead_code)]
    Keystone,
}

#[derive(Resource)]
pub struct ScriptExecutor {
    runner: Box<dyn ScriptRunner>,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self::new(Box::<RhaiScriptExecutor>::default())
    }
}

impl ScriptExecutor {
    pub fn new(runner: Box<dyn ScriptRunner>) -> Self {
        Self { runner }
    }

    pub fn run(
        &self,
        language: Language,
        source: &str,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        match language {
            Language::Rhai => self.runner.run(source),
            Language::Keystone => {
                // Execute Keystone script
                Err(ScriptExecutionError::UnsupportedLanguage(
                    "Keystone scripting is not yet implemented".to_string(),
                ))
            }
        }
    }
}

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScriptExecutor::default());
    }
}
