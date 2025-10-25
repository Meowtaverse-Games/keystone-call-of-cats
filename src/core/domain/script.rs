use crate::core::boundary::{MoveDirection, ScriptCommand, ScriptExecutionError, ScriptRunner};
use rhai::{Array, Dynamic, Engine, EvalAltResult, FLOAT as RhaiFloat, ImmutableString, Position};

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

        let mut engine = Engine::new();
        register_commands(&mut engine);

        let value = engine.eval::<Dynamic>(script).map_err(map_engine_error)?;

        if let Some(command) = value.clone().try_cast::<CommandValue>() {
            return Ok(vec![command.0]);
        }

        if let Some(array) = value.try_cast::<Array>() {
            return convert_array(array);
        }

        Err(ScriptExecutionError::InvalidCommand(
            "スクリプトは命令または命令の配列を返す必要があります。".to_string(),
        ))
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
struct CommandValue(ScriptCommand);

fn register_commands(engine: &mut Engine) {
    engine.register_type_with_name::<CommandValue>("Command");

    engine.register_fn("move_left", || {
        CommandValue(ScriptCommand::Move(MoveDirection::Left))
    });
    engine.register_fn("move_right", || {
        CommandValue(ScriptCommand::Move(MoveDirection::Right))
    });
    engine.register_fn("move_top", || {
        CommandValue(ScriptCommand::Move(MoveDirection::Top))
    });
    engine.register_fn("move_down", || {
        CommandValue(ScriptCommand::Move(MoveDirection::Down))
    });
    engine.register_fn("move", move_named);
    engine.register_fn("sleep", sleep_for);
}

fn move_named(direction: ImmutableString) -> Result<CommandValue, Box<EvalAltResult>> {
    MoveDirection::from_str(direction.as_str())
        .map(|dir| CommandValue(ScriptCommand::Move(dir)))
        .ok_or_else(|| {
            EvalAltResult::ErrorRuntime(
                format!("move命令にはleft/top/right/downのいずれかを指定してください: {direction}")
                    .into(),
                Position::NONE,
            )
            .into()
        })
}

fn sleep_for(duration: RhaiFloat) -> Result<CommandValue, Box<EvalAltResult>> {
    if duration < 0.0 {
        return Err(EvalAltResult::ErrorRuntime(
            "sleep命令の秒数は0以上である必要があります。".into(),
            Position::NONE,
        )
        .into());
    }

    Ok(CommandValue(ScriptCommand::Sleep(duration as f32)))
}

fn convert_array(array: Array) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
    let mut commands = Vec::with_capacity(array.len());
    for (index, value) in array.into_iter().enumerate() {
        let Some(command) = value.try_cast::<CommandValue>() else {
            return Err(ScriptExecutionError::InvalidCommand(format!(
                "{}番目の要素は命令ではありません。",
                index + 1
            )));
        };
        commands.push(command.0);
    }
    Ok(commands)
}

fn map_engine_error(error: Box<EvalAltResult>) -> ScriptExecutionError {
    ScriptExecutionError::Engine(error.to_string())
}
