use crate::util::script_types::{
    MoveDirection, PLAYER_TOUCHED_STATE_KEY, ScriptCommand, ScriptExecutionError, ScriptProgram,
    ScriptRunner, ScriptState, ScriptStateValue, ScriptStepper,
};
use rhai::{Dynamic, Engine, EvalAltResult, FLOAT as RhaiFloat, INT as RhaiInt, Position};
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::JoinHandle,
    time::Duration,
};

/// Rhai-based implementation of the `ScriptRunner` boundary.
pub struct RhaiScriptExecutor;

impl RhaiScriptExecutor {
    pub fn new() -> Self {
        Self
    }

    fn parse_commands(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        let script = source.trim();

        let emitter = CommandEmitter::recorder(MAX_COMMANDS);
        let state = SharedScriptState::default();
        let mut engine = base_engine(Some(MAX_OPS as u64));
        register_commands(&mut engine, emitter.clone(), state, allowed_commands);

        let _ = engine
            .eval::<Dynamic>(script)
            .map_err(|err| map_engine_error(*err))?;

        drop(engine);
        emitter
            .into_commands()
            .ok_or_else(|| ScriptExecutionError::Engine("Failed to collect commands".to_string()))
    }

    fn preflight(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<(), ScriptExecutionError> {
        let script = source.trim();
        let emitter = CommandEmitter::recorder(PREFLIGHT_MAX_COMMANDS);
        let state = SharedScriptState::default();
        let mut engine = base_engine(Some(PREFLIGHT_MAX_OPS as u64));
        register_commands(&mut engine, emitter.clone(), state, allowed_commands);

        match engine.eval::<Dynamic>(script) {
            Ok(_) => Ok(()),
            Err(err) => match *err {
                EvalAltResult::ErrorTooManyOperations(..) => Ok(()),
                EvalAltResult::ErrorRuntime(ref token, ..)
                    if token.to_string().starts_with(COMMAND_LIMIT_PREFIX) =>
                {
                    Ok(())
                }
                other => Err(map_engine_error(other)),
            },
        }
    }
}

impl Default for RhaiScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunner for RhaiScriptExecutor {
    fn run(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        self.parse_commands(source, allowed_commands)
    }
}

impl ScriptStepper for RhaiScriptExecutor {
    fn compile_step(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        self.preflight(source, allowed_commands)?;
        Ok(Box::new(RhaiScriptProgram::spawn(
            source.trim().to_string(),
            allowed_commands.cloned(),
        )?))
    }
}

#[derive(Clone)]
struct CommandValue();

const INVALID_MOVE_PREFIX: &str = "__invalid_move__:";
const INVALID_DIG_PREFIX: &str = "__invalid_dig__:";
const INVALID_SLEEP_PREFIX: &str = "__invalid_sleep__:";
const COMMAND_LIMIT_PREFIX: &str = "__command_limit__:";
const STOP_REQUEST_TOKEN: &str = "__stop_requested__";

#[derive(Clone)]
struct CommandEmitter {
    target: CommandEmitterTarget,
}

#[derive(Clone, Default)]
struct SharedScriptState {
    inner: Arc<Mutex<ScriptState>>,
}

impl SharedScriptState {
    fn write(&self, state: &ScriptState) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner = state.clone();
        }
    }

    fn touched(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|state| {
                state
                    .get(PLAYER_TOUCHED_STATE_KEY)
                    .and_then(ScriptStateValue::as_bool)
            })
            .unwrap_or(false)
    }
}

#[derive(Clone)]
enum CommandEmitterTarget {
    Recorder(CommandRecorder),
    Stream(CommandStream),
}

impl CommandEmitter {
    fn recorder(max: usize) -> Self {
        Self {
            target: CommandEmitterTarget::Recorder(CommandRecorder::with_limit(max)),
        }
    }

    fn stream(
        sender: SyncSender<ScriptCommand>,
        stop_flag: Arc<AtomicBool>,
        resume: Arc<Mutex<Receiver<()>>>,
    ) -> Self {
        Self {
            target: CommandEmitterTarget::Stream(CommandStream {
                sender,
                stop_flag,
                resume,
            }),
        }
    }

    fn emit(&self, command: ScriptCommand) -> Result<(), Box<EvalAltResult>> {
        match &self.target {
            CommandEmitterTarget::Recorder(recorder) => recorder.push(command),
            CommandEmitterTarget::Stream(stream) => stream.send(command),
        }
    }

    fn into_commands(self) -> Option<Vec<ScriptCommand>> {
        match self.target {
            CommandEmitterTarget::Recorder(recorder) => Some(recorder.into_commands()),
            CommandEmitterTarget::Stream(_) => None,
        }
    }
}

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

    fn push(&self, command: ScriptCommand) -> Result<(), Box<EvalAltResult>> {
        if let Ok(mut commands) = self.inner.lock() {
            if commands.len() >= self.max {
                return Err(EvalAltResult::ErrorRuntime(
                    format!("{COMMAND_LIMIT_PREFIX}{}", self.max).into(),
                    Position::NONE,
                )
                .into());
            }
            commands.push(command);
        }
        Ok(())
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

#[derive(Clone)]
struct CommandStream {
    sender: SyncSender<ScriptCommand>,
    stop_flag: Arc<AtomicBool>,
    resume: Arc<Mutex<Receiver<()>>>,
}

impl CommandStream {
    fn send(&self, command: ScriptCommand) -> Result<(), Box<EvalAltResult>> {
        if self.stop_flag.load(Ordering::Relaxed) {
            return Err(Box::new(EvalAltResult::ErrorRuntime(
                STOP_REQUEST_TOKEN.into(),
                Position::NONE,
            )));
        }

        self.sender.send(command).map_err(|_| {
            Box::new(EvalAltResult::ErrorRuntime(
                STOP_REQUEST_TOKEN.into(),
                Position::NONE,
            ))
        })?;

        wait_for_resume(&self.resume, &self.stop_flag)
    }
}

fn wait_for_resume(
    resume: &Arc<Mutex<Receiver<()>>>,
    stop_flag: &Arc<AtomicBool>,
) -> Result<(), Box<EvalAltResult>> {
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            return Err(Box::new(EvalAltResult::ErrorRuntime(
                STOP_REQUEST_TOKEN.into(),
                Position::NONE,
            )));
        }

        let receiver = resume.lock().map_err(|_| {
            Box::new(EvalAltResult::ErrorRuntime(
                STOP_REQUEST_TOKEN.into(),
                Position::NONE,
            ))
        })?;
        match receiver.recv_timeout(Duration::from_millis(5)) {
            Ok(_) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err(Box::new(EvalAltResult::ErrorRuntime(
                    STOP_REQUEST_TOKEN.into(),
                    Position::NONE,
                )));
            }
        }
    }
}

fn base_engine(max_operations: Option<u64>) -> Engine {
    let mut engine = Engine::new();
    if let Some(max_ops) = max_operations {
        engine.set_max_operations(max_ops);
    }
    engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_EXPR_DEPTH);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine
}

fn streaming_engine(stop_flag: &Arc<AtomicBool>) -> Engine {
    let mut engine = base_engine(None);
    let stop_flag = stop_flag.clone();
    engine.on_progress(move |_| {
        if stop_flag.load(Ordering::Relaxed) {
            Some(STOP_REQUEST_TOKEN.into())
        } else {
            None
        }
    });
    engine
}

fn register_commands(
    engine: &mut Engine,
    emitter: CommandEmitter,
    state: SharedScriptState,
    allowed_commands: Option<&HashSet<String>>,
) {
    engine.register_type_with_name::<CommandValue>("Command");

    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_left", move || {
                record_move(&emitter, MoveDirection::Left)
            });
        } else {
            engine.register_fn(
                "move_left",
                || -> Result<CommandValue, Box<EvalAltResult>> { Ok(CommandValue()) },
            );
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_right", move || {
                record_move(&emitter, MoveDirection::Right)
            });
        } else {
            engine.register_fn(
                "move_right",
                || -> Result<CommandValue, Box<EvalAltResult>> { Ok(CommandValue()) },
            );
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_top", move || {
                record_move(&emitter, MoveDirection::Top)
            });
        } else {
            engine.register_fn(
                "move_top",
                || -> Result<CommandValue, Box<EvalAltResult>> { Ok(CommandValue()) },
            );
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_down", move || {
                record_move(&emitter, MoveDirection::Down)
            });
        } else {
            engine.register_fn(
                "move_down",
                || -> Result<CommandValue, Box<EvalAltResult>> { Ok(CommandValue()) },
            );
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move", move |direction: &str| {
                move_named(direction, &emitter)
            });
        } else {
            engine.register_fn(
                "move",
                move |_: &str| -> Result<CommandValue, Box<EvalAltResult>> { Ok(CommandValue()) },
            );
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("sleep")) {
            engine.register_fn("sleep", move |duration: RhaiFloat| {
                sleep_for(duration, &emitter)
            });
        } else {
            engine.register_fn(
                "sleep",
                move |_: RhaiFloat| -> Result<CommandValue, Box<EvalAltResult>> {
                    Ok(CommandValue())
                },
            );
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("sleep")) {
            engine.register_fn("sleep", move |duration: RhaiInt| {
                sleep_for(duration as RhaiFloat, &emitter)
            });
        } else {
            engine.register_fn(
                "sleep",
                move |_: RhaiInt| -> Result<CommandValue, Box<EvalAltResult>> {
                    Ok(CommandValue())
                },
            );
        }
    }
    {
        let state = state.clone();
        if allowed_commands.is_none_or(|s| s.contains("is_touched")) {
            engine.register_fn("is_touched", move || state.touched());
        } else {
            engine.register_fn("is_touched", move || -> bool { false });
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("dig")) {
            engine.register_fn("dig", move |direction: &str| dig_named(direction, &emitter));
        } else {
            engine.register_fn(
                "dig",
                move |_: &str| -> Result<CommandValue, Box<EvalAltResult>> { Ok(CommandValue()) },
            );
        }
    }
    {
        let state = state.clone();
        if allowed_commands.is_none_or(|s| s.contains("is_empty")) {
            engine.register_fn("is_empty", move |direction: &str| {
                let key = format!("is-empty-{}", direction.to_ascii_lowercase());
                state
                    .inner
                    .lock()
                    .ok()
                    .and_then(|s| s.get(&key).and_then(|v| v.as_bool()))
                    .unwrap_or(false)
            });
        } else {
            engine.register_fn("is_empty", move |_: &str| -> bool { false });
        }
    }
}

fn record_dig(
    emitter: &CommandEmitter,
    direction: MoveDirection,
) -> Result<CommandValue, Box<EvalAltResult>> {
    let command = ScriptCommand::Dig(direction);
    emitter.emit(command)?;
    Ok(CommandValue())
}

fn dig_named(
    direction: &str,
    emitter: &CommandEmitter,
) -> Result<CommandValue, Box<EvalAltResult>> {
    match MoveDirection::from_str(direction) {
        Some(dir) => record_dig(emitter, dir),
        None => Err(EvalAltResult::ErrorRuntime(
            format!("{INVALID_DIG_PREFIX}{direction}").into(),
            Position::NONE,
        )
        .into()),
    }
}

fn record_move(
    emitter: &CommandEmitter,
    direction: MoveDirection,
) -> Result<CommandValue, Box<EvalAltResult>> {
    let command = ScriptCommand::Move(direction);
    emitter.emit(command)?;
    Ok(CommandValue())
}

fn move_named(
    direction: &str,
    emitter: &CommandEmitter,
) -> Result<CommandValue, Box<EvalAltResult>> {
    match MoveDirection::from_str(direction) {
        Some(dir) => record_move(emitter, dir),
        None => Err(EvalAltResult::ErrorRuntime(
            format!("{INVALID_MOVE_PREFIX}{direction}").into(),
            Position::NONE,
        )
        .into()),
    }
}

fn sleep_for(
    duration: RhaiFloat,
    emitter: &CommandEmitter,
) -> Result<CommandValue, Box<EvalAltResult>> {
    if duration < 0.0 {
        return Err(EvalAltResult::ErrorRuntime(
            format!("{INVALID_SLEEP_PREFIX}{duration}").into(),
            Position::NONE,
        )
        .into());
    }

    let command = ScriptCommand::Sleep(duration as f32);
    emitter.emit(command.clone())?;
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
            } else if let Some(direction) = message.strip_prefix(INVALID_DIG_PREFIX) {
                ScriptExecutionError::InvalidMoveDirection {
                    // Recycle error or add new one? Reusing for now as it's just invalid direction string
                    direction: direction.to_string(),
                }
            } else if message.starts_with(INVALID_SLEEP_PREFIX) {
                ScriptExecutionError::InvalidSleepDuration
            } else if let Some(limit) = message.strip_prefix(COMMAND_LIMIT_PREFIX) {
                let limit = limit.parse::<usize>().unwrap_or(MAX_COMMANDS);
                ScriptExecutionError::Engine(format!(
                    "Too many commands emitted (>{}). Add yields/sleeps or reduce loop counts.",
                    limit
                ))
            } else {
                ScriptExecutionError::Engine(message)
            }
        }
        other => ScriptExecutionError::Engine(other.to_string()),
    }
}

// --------- Limits & defaults ---------
const MAX_OPS: usize = 100_000; // max Rhai VM operations per evaluation (buffered)
const PREFLIGHT_MAX_OPS: usize = 100_000; // ops limit for quick validation
const PREFLIGHT_MAX_COMMANDS: usize = 512; // cap preview commands to avoid long scans
const MAX_EXPR_DEPTH: usize = 64; // max expression depth
const MAX_CALL_LEVELS: usize = 32; // max call stack depth
const MAX_COMMANDS: usize = 5_000; // cap recorded commands to prevent OOM
const STREAM_CHANNEL_SIZE: usize = 1; // backpressure so scripts yield one step at a time

// --------- Step program implementation ---------
struct RhaiScriptProgram {
    receiver: Mutex<Receiver<ScriptCommand>>,
    stop_flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    resume_tx: SyncSender<()>,
    shared_state: SharedScriptState,
}

impl RhaiScriptProgram {
    fn spawn(
        source: String,
        allowed_commands: Option<HashSet<String>>,
    ) -> Result<Self, ScriptExecutionError> {
        let (sender, receiver) = mpsc::sync_channel::<ScriptCommand>(STREAM_CHANNEL_SIZE);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (resume_tx, resume_rx) = mpsc::sync_channel::<()>(1);
        let resume_rx = Arc::new(Mutex::new(resume_rx));
        let shared_state = SharedScriptState::default();

        let mut engine = streaming_engine(&stop_flag);
        let emitter = CommandEmitter::stream(sender, stop_flag.clone(), resume_rx.clone());
        register_commands(
            &mut engine,
            emitter,
            shared_state.clone(),
            allowed_commands.as_ref(),
        );

        let ast = engine
            .compile(source.as_str())
            .map_err(|err| ScriptExecutionError::Engine(err.to_string()))?;

        let handle = std::thread::spawn({
            let resume = resume_rx.clone();
            let stop_flag = stop_flag.clone();
            move || {
                if wait_for_resume(&resume, &stop_flag).is_err() {
                    return;
                }

                let result = engine.eval_ast::<Dynamic>(&ast);
                if let Err(err) = result {
                    eprintln!("Script execution stopped: {}", map_engine_error(*err));
                }
            }
        });

        Ok(Self {
            receiver: Mutex::new(receiver),
            stop_flag,
            handle: Some(handle),
            resume_tx,
            shared_state,
        })
    }

    fn stop_and_join(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl ScriptProgram for RhaiScriptProgram {
    fn next(&mut self, state: &ScriptState) -> Option<ScriptCommand> {
        self.shared_state.write(state);

        match self.resume_tx.try_send(()) {
            Ok(_) | Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => return None,
        }

        let result = {
            let receiver = match self.receiver.lock() {
                Ok(receiver) => receiver,
                Err(poisoned) => poisoned.into_inner(),
            };
            receiver.recv_timeout(Duration::from_millis(1))
        };

        match result {
            Ok(command) => Some(command),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                self.stop_and_join();
                None
            }
        }
    }
}

impl Drop for RhaiScriptProgram {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::script_types::MoveDirection;
    use std::thread;

    #[test]
    fn touched_reflects_latest_state_between_steps() {
        let executor = RhaiScriptExecutor::new();
        let mut program = executor
            .compile_step(r#"loop { if is_touched() { move_down(); } }"#, None)
            .expect("script should compile");

        let mut touched_state = ScriptState::default();
        touched_state.insert(
            PLAYER_TOUCHED_STATE_KEY.to_string(),
            ScriptStateValue::Bool(true),
        );

        // First tick should see `touched = true` and emit a move command.
        let command = program.next(&touched_state);
        match command {
            Some(ScriptCommand::Move(MoveDirection::Down)) => {}
            other => panic!("expected move down, got {other:?}"),
        }

        // Resume the script with `touched = false`; it should stop emitting commands.
        let mut untouched_state = ScriptState::default();
        untouched_state.insert(
            PLAYER_TOUCHED_STATE_KEY.to_string(),
            ScriptStateValue::Bool(false),
        );

        // Allow the worker thread to run a few frames; it must not produce more moves.
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(2));
            let next = program.next(&untouched_state);
            assert!(
                next.is_none(),
                "touched=false should yield no commands, got {next:?}"
            );
        }
    }
}
