use crate::util::script_types::{
    MoveDirection, ScriptCommand, ScriptExecutionError, ScriptProgram, ScriptRunner, ScriptStepper,
};
use rhai::{Dynamic, Engine, EvalAltResult, FLOAT as RhaiFloat, Position};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread::JoinHandle,
};

/// Rhai-based implementation of the `ScriptRunner` boundary.
pub struct RhaiScriptExecutor;

impl RhaiScriptExecutor {
    pub fn new() -> Self {
        Self
    }

    fn parse_commands(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        let script = source.trim();

        let emitter = CommandEmitter::recorder(MAX_COMMANDS);
        let mut engine = base_engine(Some(MAX_OPS as u64));
        register_commands(&mut engine, emitter.clone());

        let _ = engine
            .eval::<Dynamic>(script)
            .map_err(|err| map_engine_error(*err))?;

        drop(engine);
        emitter
            .into_commands()
            .ok_or_else(|| ScriptExecutionError::Engine("Failed to collect commands".to_string()))
    }

    fn preflight(&self, source: &str) -> Result<(), ScriptExecutionError> {
        let script = source.trim();
        let emitter = CommandEmitter::recorder(PREFLIGHT_MAX_COMMANDS);
        let mut engine = base_engine(Some(PREFLIGHT_MAX_OPS as u64));
        register_commands(&mut engine, emitter.clone());

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
    fn run(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        self.parse_commands(source)
    }
}

impl ScriptStepper for RhaiScriptExecutor {
    fn compile_step(&self, source: &str) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        self.preflight(source)?;
        Ok(Box::new(RhaiScriptProgram::spawn(
            source.trim().to_string(),
        )?))
    }
}

#[derive(Clone)]
struct CommandValue();

const INVALID_MOVE_PREFIX: &str = "__invalid_move__:";
const INVALID_SLEEP_PREFIX: &str = "__invalid_sleep__:";
const COMMAND_LIMIT_PREFIX: &str = "__command_limit__:";
const STOP_REQUEST_TOKEN: &str = "__stop_requested__";

#[derive(Clone)]
struct CommandEmitter {
    target: CommandEmitterTarget,
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
        permit: Receiver<()>,
    ) -> Self {
        Self {
            target: CommandEmitterTarget::Stream(CommandStream {
                sender,
                stop_flag,
                permit: Arc::new(Mutex::new(permit)),
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
    permit: Arc<Mutex<Receiver<()>>>,
}

impl CommandStream {
    fn send(&self, command: ScriptCommand) -> Result<(), Box<EvalAltResult>> {
        // Wait for an explicit permit from the host, but allow cancellation.
        loop {
            if self.stop_flag.load(Ordering::Relaxed) {
                return Err(
                    EvalAltResult::ErrorRuntime(STOP_REQUEST_TOKEN.into(), Position::NONE).into(),
                );
            }

            let permit = self.permit.lock().map_err(|_| {
                EvalAltResult::ErrorRuntime(STOP_REQUEST_TOKEN.into(), Position::NONE)
            })?;
            match permit.try_recv() {
                Ok(_) => break,
                Err(TryRecvError::Empty) => {
                    std::thread::yield_now();
                    continue;
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(EvalAltResult::ErrorRuntime(
                        STOP_REQUEST_TOKEN.into(),
                        Position::NONE,
                    )
                    .into());
                }
            }
        }

        if self.stop_flag.load(Ordering::Relaxed) {
            return Err(
                EvalAltResult::ErrorRuntime(STOP_REQUEST_TOKEN.into(), Position::NONE).into(),
            );
        }

        self.sender.send(command).map_err(|_| {
            EvalAltResult::ErrorRuntime(STOP_REQUEST_TOKEN.into(), Position::NONE).into()
        })
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

fn register_commands(engine: &mut Engine, emitter: CommandEmitter) {
    engine.register_type_with_name::<CommandValue>("Command");

    {
        let emitter = emitter.clone();
        engine.register_fn("move_left", move || {
            record_move(&emitter, MoveDirection::Left)
        });
    }
    {
        let emitter = emitter.clone();
        engine.register_fn("move_right", move || {
            record_move(&emitter, MoveDirection::Right)
        });
    }
    {
        let emitter = emitter.clone();
        engine.register_fn("move_top", move || {
            record_move(&emitter, MoveDirection::Top)
        });
    }
    {
        let emitter = emitter.clone();
        engine.register_fn("move_down", move || {
            record_move(&emitter, MoveDirection::Down)
        });
    }
    {
        let emitter = emitter.clone();
        engine.register_fn("move", move |direction: &str| {
            move_named(direction, &emitter)
        });
    }
    {
        let emitter = emitter.clone();
        engine.register_fn("sleep", move |duration: RhaiFloat| {
            sleep_for(duration, &emitter)
        });
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
    permit_tx: SyncSender<()>,
}

impl RhaiScriptProgram {
    fn spawn(source: String) -> Result<Self, ScriptExecutionError> {
        let (sender, receiver) = mpsc::sync_channel::<ScriptCommand>(STREAM_CHANNEL_SIZE);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (permit_tx, permit_rx) = mpsc::sync_channel::<()>(1);

        let mut engine = streaming_engine(&stop_flag);
        let emitter = CommandEmitter::stream(sender, stop_flag.clone(), permit_rx);
        register_commands(&mut engine, emitter);

        let ast = engine
            .compile(source.as_str())
            .map_err(|err| ScriptExecutionError::Engine(err.to_string()))?;

        let handle = std::thread::spawn(move || {
            let result = engine.eval_ast::<Dynamic>(&ast);
            if let Err(err) = result {
                eprintln!("Script execution stopped: {}", map_engine_error(*err));
            }
        });

        Ok(Self {
            receiver: Mutex::new(receiver),
            stop_flag,
            handle: Some(handle),
            permit_tx,
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
    fn next(&mut self) -> Option<ScriptCommand> {
        let result = {
            let receiver = match self.receiver.lock() {
                Ok(receiver) => receiver,
                Err(poisoned) => poisoned.into_inner(),
            };
            receiver.try_recv()
        };

        match result {
            Ok(command) => Some(command),
            Err(TryRecvError::Empty) => {
                let _ = self.permit_tx.try_send(());

                let result = {
                    let receiver = match self.receiver.lock() {
                        Ok(receiver) => receiver,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    receiver.try_recv()
                };

                match result {
                    Ok(command) => Some(command),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => {
                        self.stop_and_join();
                        None
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
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
