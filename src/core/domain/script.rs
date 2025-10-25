use crate::core::boundary::{MoveDirection, ScriptCommand, ScriptExecutionError, ScriptRunner};
use rhai::{Dynamic, Engine, EvalAltResult, FLOAT as RhaiFloat, Position};
use std::sync::{Arc, Mutex};

/// Rhai-based implementation of the `ScriptRunner` boundary.
pub struct ScriptExecutor;

impl ScriptExecutor {
    pub fn new() -> Self {
        Self
    }

    fn parse_commands(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        let script = source.trim();
        if script.is_empty() {
            return Err(ScriptExecutionError::EmptyScript);
        }

        let recorder = CommandRecorder::default();
        let mut engine = Engine::new();
        register_commands(&mut engine, recorder.clone());

        let _ = engine.eval::<Dynamic>(script).map_err(map_engine_error)?;

        drop(engine);
        Ok(recorder.into_commands())
    }
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunner for ScriptExecutor {
    fn run(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        self.parse_commands(source)
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct CommandValue(ScriptCommand);

#[derive(Clone, Default)]
struct CommandRecorder(Arc<Mutex<Vec<ScriptCommand>>>);

impl CommandRecorder {
    fn push(&self, command: ScriptCommand) {
        if let Ok(mut commands) = self.0.lock() {
            commands.push(command);
        }
    }

    fn into_commands(self) -> Vec<ScriptCommand> {
        match Arc::try_unwrap(self.0) {
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
    CommandValue(command)
}

fn move_named(
    direction: &str,
    recorder: &CommandRecorder,
) -> Result<CommandValue, Box<EvalAltResult>> {
    MoveDirection::from_str(direction)
        .map(|dir| record_move(recorder, dir))
        .ok_or_else(|| {
            EvalAltResult::ErrorRuntime(
                format!("move命令にはleft/top/right/downのいずれかを指定してください: {direction}")
                    .into(),
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
            "sleep命令の秒数は0以上である必要があります。".into(),
            Position::NONE,
        )
        .into());
    }

    let command = ScriptCommand::Sleep(duration as f32);
    recorder.push(command.clone());
    Ok(CommandValue(command))
}

fn map_engine_error(error: Box<EvalAltResult>) -> ScriptExecutionError {
    ScriptExecutionError::Engine(error.to_string())
}
