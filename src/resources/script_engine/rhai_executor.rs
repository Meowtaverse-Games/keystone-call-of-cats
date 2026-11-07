use crate::util::script_types::{
    MoveDirection, PLAYER_TOUCHED_STATE_KEY, ScriptCommand, ScriptExecutionError, ScriptProgram,
    ScriptRunner, ScriptState, ScriptStateValue, ScriptStepper,
};
use rhai::{Dynamic, Engine, EvalAltResult, FLOAT as RhaiFloat, INT as RhaiInt, Position};
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::JoinHandle,
    time::Duration,
};

type SharedSignals = Arc<Mutex<HashSet<String>>>;

/// Rhai-based implementation of the `ScriptRunner` boundary.
pub struct RhaiScriptExecutor {
    blocking_steps: bool,
    shared_signals: SharedSignals,
}

impl RhaiScriptExecutor {
    pub fn new() -> Self {
        Self {
            blocking_steps: false,
            shared_signals: SharedSignals::default(),
        }
    }

    /// Wait for a VM command/error rather than advancing a simulation frame on a
    /// wall-clock timeout. Normal interactive execution retains its 1ms timeout.
    pub(crate) fn deterministic() -> Self {
        Self {
            blocking_steps: true,
            shared_signals: SharedSignals::default(),
        }
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
        register_commands(
            &mut engine,
            emitter.clone(),
            state,
            SharedSignals::default(),
            allowed_commands,
        );

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
        if self.blocking_steps {
            disable_wall_clock(&mut engine);
        }
        // Preflight must not consume or leave signals that the real run depends on.
        register_commands(
            &mut engine,
            emitter.clone(),
            state,
            SharedSignals::default(),
            allowed_commands,
        );

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
            self.blocking_steps,
            self.shared_signals.clone(),
        )?))
    }

    fn reset_signals(&self) {
        if let Ok(mut signals) = self.shared_signals.lock() {
            signals.clear();
        }
    }
}

#[derive(Clone)]
struct CommandValue();

/// What the VM thread hands back for one step.
enum StepEvent {
    Command(ScriptCommand),
    /// Blocking steps only: the VM has nothing to do this frame, so `next` must
    /// return instead of holding the frame until another stone sends a signal.
    Waiting,
}

const INVALID_MOVE_PREFIX: &str = "__invalid_move__:";
const INVALID_DIG_PREFIX: &str = "__invalid_dig__:";
const INVALID_SLEEP_PREFIX: &str = "__invalid_sleep__:";
const COMMAND_LIMIT_PREFIX: &str = "__command_limit__:";
const COMMAND_NOT_ALLOWED_PREFIX: &str = "__command_not_allowed__:";
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
        sender: SyncSender<StepEvent>,
        stop_flag: Arc<AtomicBool>,
        resume: Arc<Mutex<Receiver<()>>>,
        blocking_steps: bool,
    ) -> Self {
        Self {
            target: CommandEmitterTarget::Stream(CommandStream {
                sender,
                stop_flag,
                resume,
                blocking_steps,
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
    sender: SyncSender<StepEvent>,
    stop_flag: Arc<AtomicBool>,
    resume: Arc<Mutex<Receiver<()>>>,
    blocking_steps: bool,
}

impl CommandStream {
    fn send(&self, command: ScriptCommand) -> Result<(), Box<EvalAltResult>> {
        self.yield_event(StepEvent::Command(command))
    }

    /// Give up the current frame while `receive` has no signal to consume.
    fn wait_for_next_step(&self) -> Result<(), Box<EvalAltResult>> {
        if self.blocking_steps {
            self.yield_event(StepEvent::Waiting)
        } else {
            wait_for_resume(&self.resume, &self.stop_flag)
        }
    }

    fn yield_event(&self, event: StepEvent) -> Result<(), Box<EvalAltResult>> {
        if self.stop_flag.load(Ordering::Relaxed) {
            return Err(Box::new(EvalAltResult::ErrorRuntime(
                STOP_REQUEST_TOKEN.into(),
                Position::NONE,
            )));
        }

        self.sender.send(event).map_err(|_| {
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

/// Native override also covers dynamically constructed Fn pointers and eval.
fn disable_wall_clock(engine: &mut Engine) {
    engine.register_fn("timestamp", || {
        command_not_allowed_error::<Dynamic>(
            "timestamp (wall clock is unavailable in headless verification)",
        )
    });
}

fn register_commands(
    engine: &mut Engine,
    emitter: CommandEmitter,
    state: SharedScriptState,
    signals: SharedSignals,
    allowed_commands: Option<&HashSet<String>>,
) {
    engine.register_type_with_name::<CommandValue>("Command");

    {
        let send = signals.clone();
        engine.register_fn("send", move |channel: &str| send_signal(channel, &send));
        let send = signals.clone();
        engine.register_fn("send", move |channel: RhaiInt| {
            send_signal(&channel.to_string(), &send)
        });
    }
    {
        let receive_emitter = emitter.clone();
        let receive = signals.clone();
        engine.register_fn("receive", move |channel: &str| {
            receive_signal(channel, &receive, &receive_emitter)
        });
        let emitter = emitter.clone();
        let receive = signals;
        engine.register_fn("receive", move |channel: RhaiInt| {
            receive_signal(&channel.to_string(), &receive, &emitter)
        });
    }

    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_left", move || {
                record_move(&emitter, MoveDirection::Left)
            });
        } else {
            engine.register_fn("move_left", || command_not_allowed("move"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_right", move || {
                record_move(&emitter, MoveDirection::Right)
            });
        } else {
            engine.register_fn("move_right", || command_not_allowed("move"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_top", move || {
                record_move(&emitter, MoveDirection::Top)
            });
        } else {
            engine.register_fn("move_top", || command_not_allowed("move"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move_down", move || {
                record_move(&emitter, MoveDirection::Down)
            });
        } else {
            engine.register_fn("move_down", || command_not_allowed("move"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("move")) {
            engine.register_fn("move", move |direction: &str| {
                move_named(direction, &emitter)
            });
        } else {
            engine.register_fn("move", |_: &str| command_not_allowed("move"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("sleep")) {
            engine.register_fn("sleep", move |duration: RhaiFloat| {
                sleep_for(duration, &emitter)
            });
        } else {
            engine.register_fn("sleep", |_: RhaiFloat| command_not_allowed("sleep"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("sleep")) {
            engine.register_fn("sleep", move |duration: RhaiInt| {
                sleep_for(duration as RhaiFloat, &emitter)
            });
        } else {
            engine.register_fn("sleep", |_: RhaiInt| command_not_allowed("sleep"));
        }
    }
    {
        let state = state.clone();
        if allowed_commands.is_none_or(|s| s.contains("is_touched")) {
            engine.register_fn("is_touched", move || state.touched());
        } else {
            engine.register_fn("is_touched", || sensor_not_allowed("is_touched"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("dig")) {
            engine.register_fn("dig", move |direction: &str| dig_named(direction, &emitter));
        } else {
            engine.register_fn("dig", |_: &str| command_not_allowed("dig"));
        }
    }
    {
        let emitter = emitter.clone();
        if allowed_commands.is_none_or(|s| s.contains("place")) {
            engine.register_fn("place", move |direction: &str| {
                place_named(direction, &emitter)
            });
        } else {
            engine.register_fn("place", |_: &str| command_not_allowed("place"));
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
            engine.register_fn("is_empty", |_: &str| sensor_not_allowed("is_empty"));
        }
    }
}

fn send_signal(channel: &str, signals: &SharedSignals) -> CommandValue {
    if let Ok(mut signals) = signals.lock() {
        signals.insert(channel.to_owned());
    }
    CommandValue()
}

fn receive_signal(
    channel: &str,
    signals: &SharedSignals,
    emitter: &CommandEmitter,
) -> Result<CommandValue, Box<EvalAltResult>> {
    loop {
        let received = signals
            .lock()
            .map(|mut signals| signals.remove(channel))
            .unwrap_or(false);
        if received {
            return Ok(CommandValue());
        }
        match &emitter.target {
            CommandEmitterTarget::Recorder(_) => return Ok(CommandValue()),
            CommandEmitterTarget::Stream(stream) => stream.wait_for_next_step()?,
        }
    }
}

fn command_not_allowed(command: &str) -> Result<CommandValue, Box<EvalAltResult>> {
    command_not_allowed_error(command)
}

fn sensor_not_allowed(command: &str) -> Result<bool, Box<EvalAltResult>> {
    command_not_allowed_error(command)
}

fn command_not_allowed_error<T>(command: &str) -> Result<T, Box<EvalAltResult>> {
    // A regular runtime error can be caught by a script. Capabilities are an execution
    // boundary, so use Rhai's uncatchable termination error instead.
    Err(EvalAltResult::ErrorTerminated(
        format!("{COMMAND_NOT_ALLOWED_PREFIX}{command}").into(),
        Position::NONE,
    )
    .into())
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

fn record_place(
    emitter: &CommandEmitter,
    direction: MoveDirection,
) -> Result<CommandValue, Box<EvalAltResult>> {
    emitter.emit(ScriptCommand::Place(direction))?;
    Ok(CommandValue())
}

fn place_named(
    direction: &str,
    emitter: &CommandEmitter,
) -> Result<CommandValue, Box<EvalAltResult>> {
    match MoveDirection::from_str(direction) {
        Some(dir) => record_place(emitter, dir),
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
        EvalAltResult::ErrorTerminated(value, _) => {
            let message = value.to_string();
            if let Some(command) = message.strip_prefix(COMMAND_NOT_ALLOWED_PREFIX) {
                ScriptExecutionError::InvalidCommand(format!(
                    "Command '{command}' is not allowed for this stone."
                ))
            } else if message == "Headless script step exceeded 100000 operations" {
                ScriptExecutionError::Engine(message)
            } else {
                ScriptExecutionError::Engine("Script terminated".to_string())
            }
        }
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
            } else if let Some(command) = message.strip_prefix(COMMAND_NOT_ALLOWED_PREFIX) {
                ScriptExecutionError::InvalidCommand(format!(
                    "Command '{command}' is not allowed for this stone."
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
// A state update may resume only a worker already waiting for that update. Keeping a permit here
// would let an earlier `is_touched = true` observation emit a move after contact has ended.
const RESUME_CHANNEL_SIZE: usize = 0;

// --------- Step program implementation ---------
struct RhaiScriptProgram {
    blocking_steps: bool,
    step_operations: Arc<AtomicUsize>,
    receiver: Mutex<Receiver<StepEvent>>,
    stop_flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    resume_tx: SyncSender<()>,
    shared_state: SharedScriptState,
    runtime_error: Arc<Mutex<Option<ScriptExecutionError>>>,
}

impl RhaiScriptProgram {
    fn spawn(
        source: String,
        allowed_commands: Option<HashSet<String>>,
        blocking_steps: bool,
        signals: SharedSignals,
    ) -> Result<Self, ScriptExecutionError> {
        let (sender, receiver) = mpsc::sync_channel::<StepEvent>(STREAM_CHANNEL_SIZE);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (resume_tx, resume_rx) = mpsc::sync_channel::<()>(RESUME_CHANNEL_SIZE);
        let resume_rx = Arc::new(Mutex::new(resume_rx));
        let shared_state = SharedScriptState::default();
        let runtime_error = Arc::new(Mutex::new(None));

        let mut engine = streaming_engine(&stop_flag);
        let step_operations = Arc::new(AtomicUsize::new(0));
        if blocking_steps {
            disable_wall_clock(&mut engine);
            let operations = step_operations.clone();
            let stop = stop_flag.clone();
            engine.on_progress(move |_| {
                if stop.load(Ordering::Relaxed) {
                    Some(STOP_REQUEST_TOKEN.into())
                } else if operations.fetch_add(1, Ordering::Relaxed) >= MAX_OPS {
                    Some("Headless script step exceeded 100000 operations".into())
                } else {
                    None
                }
            });
        }

        let emitter =
            CommandEmitter::stream(sender, stop_flag.clone(), resume_rx.clone(), blocking_steps);
        register_commands(
            &mut engine,
            emitter,
            shared_state.clone(),
            signals,
            allowed_commands.as_ref(),
        );

        let ast = engine
            .compile(source.as_str())
            .map_err(|err| ScriptExecutionError::Engine(err.to_string()))?;

        let handle = std::thread::spawn({
            let resume = resume_rx.clone();
            let stop_flag = stop_flag.clone();
            let runtime_error = runtime_error.clone();
            move || {
                if wait_for_resume(&resume, &stop_flag).is_err() {
                    return;
                }

                let result = engine.eval_ast::<Dynamic>(&ast);
                if let Err(err) = result {
                    let error = map_engine_error(*err);
                    if !matches!(error, ScriptExecutionError::Engine(ref message) if message == STOP_REQUEST_TOKEN)
                        && let Ok(mut stored_error) = runtime_error.lock()
                    {
                        *stored_error = Some(error);
                    }
                }
            }
        });

        Ok(Self {
            blocking_steps,
            step_operations,
            receiver: Mutex::new(receiver),
            stop_flag,
            handle: Some(handle),
            resume_tx,
            shared_state,
            runtime_error,
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
        if self.blocking_steps {
            self.step_operations.store(0, Ordering::Relaxed);
        }

        if self.blocking_steps {
            // The resume channel is a rendezvous channel. A nonblocking send can
            // miss a worker that has not reached its receive yet, leaving both
            // sides waiting forever when command reception below is blocking.
            if self.resume_tx.send(()).is_err() {
                return None;
            }
        } else {
            match self.resume_tx.try_send(()) {
                Ok(_) | Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Disconnected(_)) => return None,
            }
        }

        let result = {
            let receiver = match self.receiver.lock() {
                Ok(receiver) => receiver,
                Err(poisoned) => poisoned.into_inner(),
            };
            if self.blocking_steps {
                receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
            } else {
                receiver.recv_timeout(Duration::from_millis(1))
            }
        };

        match result {
            Ok(StepEvent::Command(command)) => Some(command),
            Ok(StepEvent::Waiting) | Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                self.stop_and_join();
                None
            }
        }
    }

    fn take_error(&mut self) -> Option<ScriptExecutionError> {
        self.runtime_error
            .lock()
            .ok()
            .and_then(|mut runtime_error| runtime_error.take())
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
    use std::{collections::HashSet, thread, time::Instant};

    fn allowed(commands: &[&str]) -> HashSet<String> {
        commands
            .iter()
            .map(|command| (*command).to_string())
            .collect()
    }

    #[test]
    fn deterministic_steps_wait_for_worker_at_startup_and_after_each_yield() {
        let (finished, result) = mpsc::channel();
        let worker = thread::spawn(move || {
            let executor = RhaiScriptExecutor::deterministic();
            for _ in 0..100 {
                let mut program = executor
                    .compile_step("move(\"down\"); move(\"up\");", None)
                    .unwrap();
                for direction in [MoveDirection::Down, MoveDirection::Top] {
                    assert!(
                        matches!(program.next(&ScriptState::new()), Some(ScriptCommand::Move(actual)) if actual == direction)
                    );
                }
                assert!(program.next(&ScriptState::new()).is_none());
                assert!(program.take_error().is_none());
            }
            finished.send(()).unwrap();
        });
        // A missed rendezvous must fail this test instead of hanging the suite.
        result
            .recv_timeout(Duration::from_secs(10))
            .expect("deterministic VM handshake stalled");
        worker.join().unwrap();
    }

    /// Fails the test instead of hanging the suite when a blocking step stalls.
    fn within_deadline(name: &str, body: impl FnOnce() + Send + 'static) {
        let (finished, result) = mpsc::channel();
        let worker = thread::spawn(move || {
            body();
            finished.send(()).unwrap();
        });
        result
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("{name} stalled"));
        worker.join().unwrap();
    }

    #[test]
    fn deterministic_receive_yields_the_frame_until_a_later_stone_sends() {
        within_deadline("deterministic receive", || {
            let executor = RhaiScriptExecutor::deterministic();
            let state = ScriptState::new();
            let mut receiver = executor
                .compile_step(r#"receive("go"); move("right");"#, None)
                .unwrap();
            let mut sender = executor
                .compile_step(r#"sleep(1); send("go"); sleep(1);"#, None)
                .unwrap();

            // Frame 1: the receiver goes first and must not block the frame.
            assert!(receiver.next(&state).is_none());
            assert!(matches!(sender.next(&state), Some(ScriptCommand::Sleep(_))));
            // Frame 2: the signal is not out yet, so the receiver still waits.
            assert!(receiver.next(&state).is_none());
            assert!(matches!(sender.next(&state), Some(ScriptCommand::Sleep(_))));
            // Frame 3: the signal arrived during frame 2.
            assert!(matches!(
                receiver.next(&state),
                Some(ScriptCommand::Move(MoveDirection::Right))
            ));
            assert!(receiver.next(&state).is_none());
            assert!(receiver.take_error().is_none());
        });
    }

    #[test]
    fn deterministic_receive_without_a_sender_keeps_waiting_and_can_be_dropped() {
        within_deadline("deterministic unanswered receive", || {
            let executor = RhaiScriptExecutor::deterministic();
            let state = ScriptState::new();
            let mut receiver = executor
                .compile_step(r#"receive("never"); move("right");"#, None)
                .unwrap();

            for _ in 0..1_000 {
                assert!(receiver.next(&state).is_none());
                assert!(receiver.take_error().is_none());
            }
        });
    }

    #[test]
    fn deterministic_signal_order_is_repeatable() {
        within_deadline("deterministic signal replay", || {
            let run = || {
                let executor = RhaiScriptExecutor::deterministic();
                let state = ScriptState::new();
                let mut programs = [
                    r#"receive("a"); move("left"); send("b"); receive("a"); move("left");"#,
                    r#"sleep(1); send("a"); receive("b"); send("a"); move("right");"#,
                ]
                .map(|source| executor.compile_step(source, None).unwrap());

                let mut trace = Vec::new();
                for frame in 0..20 {
                    for (stone, program) in programs.iter_mut().enumerate() {
                        trace.push((frame, stone, format!("{:?}", program.next(&state))));
                    }
                }
                trace
            };

            let first = run();
            assert_eq!(
                first
                    .iter()
                    .filter(|(_, _, command)| command != "None")
                    .count(),
                4
            );
            for _ in 0..50 {
                assert_eq!(run(), first);
            }
        });
    }

    #[test]
    fn deterministic_steps_reject_dynamic_wall_clock_access() {
        let executor = RhaiScriptExecutor::deterministic();
        let result = executor.compile_step("let f = Fn(\"time\" + \"stamp\"); f.call();", None);
        assert!(
            matches!(result, Err(ScriptExecutionError::InvalidCommand(message)) if message.contains("wall clock"))
        );
    }

    #[test]
    fn rejects_commands_and_sensors_outside_the_stone_capabilities() {
        let executor = RhaiScriptExecutor::new();
        let type1 = allowed(&["move", "sleep"]);

        for source in [
            "dig(\"down\");",
            "if is_touched() { move(\"right\"); }",
            "if is_empty(\"right\") { move(\"right\"); }",
            "try { dig(\"down\"); } catch (error) { move(\"right\"); }",
        ] {
            assert!(
                executor.compile_step(source, Some(&type1)).is_err(),
                "{source} should be rejected instead of silently doing nothing"
            );
        }
    }

    #[test]
    fn reports_a_runtime_error_when_a_disallowed_branch_becomes_reachable() {
        let executor = RhaiScriptExecutor::new();
        let allows_sensor_but_not_dig = allowed(&["move", "sleep", "is_touched"]);
        let mut program = executor
            .compile_step(
                r#"loop { if is_touched() { dig("down"); } }"#,
                Some(&allows_sensor_but_not_dig),
            )
            .expect("an unreachable branch can pass Rhai preflight");

        let mut untouched_state = ScriptState::default();
        untouched_state.insert(
            PLAYER_TOUCHED_STATE_KEY.to_string(),
            ScriptStateValue::Bool(false),
        );
        assert!(program.next(&untouched_state).is_none());
        assert!(program.take_error().is_none());

        let mut touched_state = ScriptState::default();
        touched_state.insert(
            PLAYER_TOUCHED_STATE_KEY.to_string(),
            ScriptStateValue::Bool(true),
        );

        let deadline = Instant::now() + Duration::from_millis(100);
        loop {
            assert!(program.next(&touched_state).is_none());
            if let Some(ScriptExecutionError::InvalidCommand(message)) = program.take_error() {
                assert!(message.contains("dig"));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "expected a disallowed command error"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn place_emits_only_when_the_capability_is_allowed() {
        let executor = RhaiScriptExecutor::new();
        let type4 = allowed(&["move", "is_touched", "is_empty", "place"]);
        assert!(matches!(
            executor.run("place(\"right\");", Some(&type4)),
            Ok(commands) if matches!(commands.as_slice(), [ScriptCommand::Place(MoveDirection::Right)])
        ));
        assert!(
            executor
                .compile_step("place(\"right\");", Some(&allowed(&["move"])))
                .is_err()
        );
    }

    fn next_command_within(
        program: &mut dyn ScriptProgram,
        state: &ScriptState,
    ) -> Option<ScriptCommand> {
        let deadline = Instant::now() + Duration::from_millis(100);
        loop {
            if let Some(command) = program.next(state) {
                return Some(command);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::yield_now();
        }
    }

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

        // `next` is a non-blocking poll; wait for the worker to receive this state update.
        let command = next_command_within(&mut *program, &touched_state);
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

    #[test]
    fn touched_loop_waits_then_moves_right_until_contact_ends() {
        let executor = RhaiScriptExecutor::new();
        let mut program = executor
            .compile_step(r#"loop { if is_touched() { move("right"); } }"#, None)
            .expect("script should compile");

        let mut untouched_state = ScriptState::default();
        untouched_state.insert(
            PLAYER_TOUCHED_STATE_KEY.to_string(),
            ScriptStateValue::Bool(false),
        );

        // The worker stays alive while the player takes time to reach the stone.
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(2));
            assert!(program.next(&untouched_state).is_none());
        }

        let mut touched_state = ScriptState::default();
        touched_state.insert(
            PLAYER_TOUCHED_STATE_KEY.to_string(),
            ScriptStateValue::Bool(true),
        );
        assert!(matches!(
            next_command_within(&mut *program, &touched_state),
            Some(ScriptCommand::Move(MoveDirection::Right))
        ));

        // After stepping off, the same running loop must not emit another move.
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(2));
            assert!(program.next(&untouched_state).is_none());
        }
    }

    fn assert_waiting(program: &mut dyn ScriptProgram, state: &ScriptState) {
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(2));
            let next = program.next(state);
            assert!(next.is_none(), "receive should block, got {next:?}");
        }
    }

    #[test]
    fn receive_waits_until_another_stone_sends() {
        let executor = RhaiScriptExecutor::new();
        let state = ScriptState::default();
        let mut receiver = executor
            .compile_step(r#"receive("go"); move("right");"#, None)
            .expect("receiver should compile");
        let mut sender = executor
            .compile_step(r#"sleep(1); send("go"); sleep(1);"#, None)
            .expect("sender should compile");

        assert_waiting(&mut *receiver, &state);

        assert!(matches!(
            next_command_within(&mut *sender, &state),
            Some(ScriptCommand::Sleep(_))
        ));
        assert_waiting(&mut *receiver, &state);

        assert!(matches!(
            next_command_within(&mut *sender, &state),
            Some(ScriptCommand::Sleep(_))
        ));
        assert!(matches!(
            next_command_within(&mut *receiver, &state),
            Some(ScriptCommand::Move(MoveDirection::Right))
        ));
    }

    #[test]
    fn receive_consumes_each_signal_once() {
        let executor = RhaiScriptExecutor::new();
        let state = ScriptState::default();
        let mut sender = executor
            .compile_step(r#"send(1); sleep(1);"#, None)
            .expect("sender should compile");
        let mut receiver = executor
            .compile_step(
                r#"receive(1); move("left"); receive(1); move("left");"#,
                None,
            )
            .expect("receiver should compile");

        assert!(next_command_within(&mut *sender, &state).is_some());
        assert!(matches!(
            next_command_within(&mut *receiver, &state),
            Some(ScriptCommand::Move(MoveDirection::Left))
        ));
        assert_waiting(&mut *receiver, &state);
    }

    #[test]
    fn signals_are_not_shared_between_executors() {
        let state = ScriptState::default();
        let mut sender = RhaiScriptExecutor::new()
            .compile_step(r#"send("go"); sleep(1);"#, None)
            .expect("sender should compile");
        let mut receiver = RhaiScriptExecutor::new()
            .compile_step(r#"receive("go"); move("right");"#, None)
            .expect("receiver should compile");

        assert!(next_command_within(&mut *sender, &state).is_some());
        assert_waiting(&mut *receiver, &state);
    }

    #[test]
    fn reset_signals_drops_signals_from_a_previous_run() {
        let executor = RhaiScriptExecutor::new();
        let state = ScriptState::default();
        let mut sender = executor
            .compile_step(r#"send("go"); sleep(1);"#, None)
            .expect("sender should compile");
        assert!(next_command_within(&mut *sender, &state).is_some());
        drop(sender);

        executor.reset_signals();
        let mut receiver = executor
            .compile_step(r#"receive("go"); move("right");"#, None)
            .expect("receiver should compile");
        assert_waiting(&mut *receiver, &state);
    }
}
