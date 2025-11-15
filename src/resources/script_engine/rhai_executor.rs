use crate::util::script_types::{
    MoveDirection, ScriptCommand, ScriptExecutionError, ScriptProgram, ScriptRunner, ScriptStepper,
};
use rhai::{Dynamic, Engine, EvalAltResult, FLOAT as RhaiFloat, Position};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Rhai-based implementation of the `ScriptRunner` boundary.
pub struct RhaiScriptExecutor;

impl RhaiScriptExecutor {
    pub fn new() -> Self {
        Self
    }

    fn parse_commands(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        let script = source.trim();
        if script.is_empty() {
            return Err(ScriptExecutionError::EmptyScript);
        }

        let recorder = CommandRecorder::with_limit(MAX_COMMANDS);
        let mut engine = Engine::new();

        // Safety rails: prevent runaway scripts
        engine.set_max_operations(MAX_OPS as u64);
        engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_EXPR_DEPTH);
        engine.set_max_call_levels(MAX_CALL_LEVELS);
        register_commands(&mut engine, recorder.clone());

        let _ = engine
            .eval::<Dynamic>(script)
            .map_err(|err| map_engine_error(*err))?;

        if recorder.exceeded_limit() {
            return Err(ScriptExecutionError::Engine(format!(
                "Too many commands emitted (>{}). Add yields/sleeps or reduce loop counts.",
                MAX_COMMANDS
            )));
        }

        drop(engine);
        Ok(recorder.into_commands())
    }
}

impl Default for RhaiScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunner for RhaiScriptExecutor {
    fn run(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        self.parse_commands(source)
    }
}

impl ScriptStepper for RhaiScriptExecutor {
    fn compile_step(&self, source: &str) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        let commands = self.parse_commands(source)?;
        Ok(Box::new(RhaiScriptProgram::new(commands)))
    }
}

#[derive(Clone)]
struct CommandValue();

const INVALID_MOVE_PREFIX: &str = "__invalid_move__:";
const INVALID_SLEEP_PREFIX: &str = "__invalid_sleep__:";

#[derive(Clone)]
struct CommandRecorder {
    inner: Arc<Mutex<Vec<ScriptCommand>>>,
    max: usize,
}

impl CommandRecorder {
    fn with_limit(max: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
            max,
        }
    }

    fn push(&self, command: ScriptCommand) {
        if let Ok(mut commands) = self.inner.lock()
            && commands.len() < self.max
        {
            commands.push(command);
        }
    }

    fn exceeded_limit(&self) -> bool {
        self.inner
            .lock()
            .map(|v| v.len() >= self.max)
            .unwrap_or(true)
    }

    fn into_commands(self) -> Vec<ScriptCommand> {
        match Arc::try_unwrap(self.inner) {
            Ok(mutex) => match mutex.into_inner() {
                Ok(commands) => commands,
                Err(poisoned) => poisoned.into_inner(),
            },
            Err(arc) => match arc.lock() {
                Ok(commands) => commands.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            },
        }
    }
}

fn register_commands(engine: &mut Engine, recorder: CommandRecorder) {
    engine.register_type_with_name::<CommandValue>("Command");

    {
        let recorder = recorder.clone();
        engine.register_fn("move_left", move || {
            record_move(&recorder, MoveDirection::Left)
        });
    }
    {
        let recorder = recorder.clone();
        engine.register_fn("move_right", move || {
            record_move(&recorder, MoveDirection::Right)
        });
    }
    {
        let recorder = recorder.clone();
        engine.register_fn("move_top", move || {
            record_move(&recorder, MoveDirection::Top)
        });
    }
    {
        let recorder = recorder.clone();
        engine.register_fn("move_down", move || {
            record_move(&recorder, MoveDirection::Down)
        });
    }
    {
        let recorder = recorder.clone();
        engine.register_fn("move", move |direction: &str| {
            move_named(direction, &recorder)
        });
    }
    {
        let recorder = recorder.clone();
        engine.register_fn("sleep", move |duration: RhaiFloat| {
            sleep_for(duration, &recorder)
        });
    }
}

fn record_move(recorder: &CommandRecorder, direction: MoveDirection) -> CommandValue {
    let command = ScriptCommand::Move(direction);
    recorder.push(command.clone());
    CommandValue()
}

fn move_named(
    direction: &str,
    recorder: &CommandRecorder,
) -> Result<CommandValue, Box<EvalAltResult>> {
    MoveDirection::from_str(direction)
        .map(|dir| record_move(recorder, dir))
        .ok_or_else(|| {
            EvalAltResult::ErrorRuntime(
                format!("{INVALID_MOVE_PREFIX}{direction}").into(),
                Position::NONE,
            )
            .into()
        })
}

fn sleep_for(
    duration: RhaiFloat,
    recorder: &CommandRecorder,
) -> Result<CommandValue, Box<EvalAltResult>> {
    if duration < 0.0 {
        return Err(EvalAltResult::ErrorRuntime(
            format!("{INVALID_SLEEP_PREFIX}{duration}").into(),
            Position::NONE,
        )
        .into());
    }

    let command = ScriptCommand::Sleep(duration as f32);
    recorder.push(command.clone());
    Ok(CommandValue())
}

fn map_engine_error(error: EvalAltResult) -> ScriptExecutionError {
    match error {
        EvalAltResult::ErrorRuntime(value, _) => {
            let message = value.to_string();
            if let Some(direction) = message.strip_prefix(INVALID_MOVE_PREFIX) {
                ScriptExecutionError::InvalidMoveDirection {
                    direction: direction.to_string(),
                }
            } else if message.starts_with(INVALID_SLEEP_PREFIX) {
                ScriptExecutionError::InvalidSleepDuration
            } else {
                ScriptExecutionError::Engine(message)
            }
        }
        other => ScriptExecutionError::Engine(other.to_string()),
    }
}

// --------- Limits & defaults ---------
const MAX_OPS: usize = 100_000; // max Rhai VM operations per evaluation
const MAX_EXPR_DEPTH: usize = 64; // max expression depth
const MAX_CALL_LEVELS: usize = 32; // max call stack depth
const MAX_COMMANDS: usize = 5_000; // cap recorded commands to prevent OOM

// --------- Step program implementation ---------
struct RhaiScriptProgram {
    queue: VecDeque<ScriptCommand>,
}

impl RhaiScriptProgram {
    fn new(commands: Vec<ScriptCommand>) -> Self {
        Self {
            queue: VecDeque::from(commands),
        }
    }
}

impl ScriptProgram for RhaiScriptProgram {
    fn next(&mut self) -> Option<ScriptCommand> {
        self.queue.pop_front()
    }
}
