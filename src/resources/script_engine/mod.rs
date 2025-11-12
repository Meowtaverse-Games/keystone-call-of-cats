mod rhai_executor;

use bevy::prelude::Resource;

pub use rhai_executor::RhaiScriptExecutor;

use crate::util::script_types::{ScriptCommand, ScriptExecutionError, ScriptRunner};

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
            Language::Keystone => Err(ScriptExecutionError::UnsupportedLanguage(
                "Keystone scripting is not yet implemented".to_string(),
            )),
        }
    }
}
