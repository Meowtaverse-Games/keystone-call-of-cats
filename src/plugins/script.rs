use crate::core::{
    boundary::{ScriptCommand, ScriptExecutionError, ScriptRunner},
    domain::script::ScriptExecutor as DomainScriptExecutor,
};
use bevy::prelude::*;

pub enum Language {
    Rhai,
    #[allow(dead_code)]
    Keystone,
}

#[derive(Resource, Default)]
pub struct ScriptExecutor(DomainScriptExecutor);

impl ScriptExecutor {
    pub fn run(
        &self,
        language: Language,
        source: &str,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        match language {
            Language::Rhai => self.0.run(source),
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
