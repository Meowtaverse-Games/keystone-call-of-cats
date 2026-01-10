mod rhai_executor;
mod keystone_executor;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

pub use rhai_executor::RhaiScriptExecutor;
pub use keystone_executor::KeystoneScriptExecutor;

use crate::util::script_types::{
    ScriptCommand, ScriptExecutionError, ScriptProgram, ScriptRunner, ScriptStepper,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Rhai,
    Keystone,
}

#[derive(Resource)]
pub struct ScriptExecutor {
    #[allow(dead_code)]
    runner: Box<dyn ScriptRunner>,
    stepper: Box<dyn ScriptStepper>,
    ks_runner: Box<dyn ScriptRunner>,
    ks_stepper: Box<dyn ScriptStepper>,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self::new(
            Box::<RhaiScriptExecutor>::default(),
            Box::<RhaiScriptExecutor>::default(),
            Box::<KeystoneScriptExecutor>::default(),
            Box::<KeystoneScriptExecutor>::default(),
        )
    }
}

impl ScriptExecutor {
    pub fn new(runner: Box<dyn ScriptRunner>, stepper: Box<dyn ScriptStepper>, ks_runner: Box<dyn ScriptRunner>, ks_stepper: Box<dyn ScriptStepper>,) -> Self {
        Self { runner, stepper, ks_runner, ks_stepper }
    }

    #[allow(dead_code)]
    pub fn run(
        &self,
        language: Language,
        source: &str,
        allowed_commands: Option<&std::collections::HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        match language {
            Language::Rhai => self.runner.run(source, allowed_commands),
            Language::Keystone => self.ks_runner.run(source, allowed_commands),
        }
    }

    pub fn compile_step(
        &self,
        language: Language,
        source: &str,
        allowed_commands: Option<&std::collections::HashSet<String>>,
    ) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        match language {
            Language::Rhai => self.stepper.compile_step(source, allowed_commands),
            Language::Keystone => self.ks_stepper.compile_step(source, allowed_commands),
        }
    }
}
