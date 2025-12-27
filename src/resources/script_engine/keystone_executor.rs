use crate::util::script_types::{
    MoveDirection, PLAYER_TOUCHED_STATE_KEY, ScriptCommand, ScriptExecutionError, ScriptProgram,
    ScriptRunner, ScriptState, ScriptStateValue, ScriptStepper,
};
use keystone_lang::*;
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

pub struct KeystoneScriptExecutor;

impl KeystoneScriptExecutor {
    pub fn new() -> Self {
        Self
    }

    fn parse_commands(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        Ok(vec![ScriptCommand::Move(MoveDirection::Right)])
    }

    fn preflight(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<(), ScriptExecutionError> {
        // let script = source.trim();
        // let emitter = CommandEmitter::recorder(PREFLIGHT_MAX_COMMANDS);
        // let state = SharedScriptState::default();
        // let mut engine = base_engine(Some(PREFLIGHT_MAX_OPS as u64));
        // register_commands(&mut engine, emitter.clone(), state, allowed_commands);

        // match engine.eval::<Dynamic>(script) {
        //     Ok(_) => Ok(()),
        //     Err(err) => match *err {
        //         EvalAltResult::ErrorTooManyOperations(..) => Ok(()),
        //         EvalAltResult::ErrorRuntime(ref token, ..)
        //             if token.to_string().starts_with(COMMAND_LIMIT_PREFIX) =>
        //         {
        //             Ok(())
        //         }
        //         other => Err(map_engine_error(other)),
        //     },
        // }
        Ok(())
    }
}

impl Default for KeystoneScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunner for KeystoneScriptExecutor {
    fn run(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        self.parse_commands(source, allowed_commands)
    }
}

impl ScriptStepper for KeystoneScriptExecutor {
    fn compile_step(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        self.preflight(source, allowed_commands)?;
        Ok(Box::new(KeystoneScriptProgram::spawn(
            source.trim().to_string(),
            allowed_commands.cloned(),
        ))?)
    }
}

#[derive(Clone)]
struct CommandValue();

const STOP_REQUEST_TOKEN: &str = "__stop_requested__";
const COMMAND_LIMIT_PREFIX: &str = "__command_limit__:";

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

    fn emit(&self, command: ScriptCommand) -> Result<(), Box<dyn std::error::Error>> {
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

    fn push(&self, command: ScriptCommand) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut commands) = self.inner.lock() {
            if commands.len() >= self.max {
                return Err(format!("{COMMAND_LIMIT_PREFIX}{}", self.max).into());
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
    fn send(&self, command: ScriptCommand) -> Result<(), Box<dyn std::error::Error>> {
        if self.stop_flag.load(Ordering::Relaxed) {
            return Err(STOP_REQUEST_TOKEN.into());
        }

        self.sender.send(command).map_err(|_| STOP_REQUEST_TOKEN.into())?;

        wait_for_resume(&self.resume, &self.stop_flag)
    }
}

fn wait_for_resume(
    resume: &Arc<Mutex<Receiver<()>>>,
    stop_flag: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            return Err(STOP_REQUEST_TOKEN.into());
        }

        let receiver = resume.lock().map_err(|_| STOP_REQUEST_TOKEN.into())?;
        match receiver.recv_timeout(Duration::from_millis(5)) {
            Ok(_) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return Err(STOP_REQUEST_TOKEN.into()),
        }
    }
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
}


struct KeystoneScriptProgram {
    receiver: Mutex<Receiver<ScriptCommand>>,
    stop_flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    resume_tx: SyncSender<()>,
    shared_state: SharedScriptState,
}

impl KeystoneScriptProgram {
    fn spawn(
        /*source: String, allowed_commands: Option<HashSet<String>>*/
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (sender, receiver) = mpsc::sync_channel::<ScriptCommand>(1);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (resume_tx, resume_rx) = mpsc::sync_channel::<()>(1);
        let resume_rx = Arc::new(Mutex::new(resume_rx));
        let shared_state = SharedScriptState::default();

        let emitter = CommandEmitter::stream(sender, stop_flag.clone(), resume_rx.clone());

        //--KEYSTONE EVAL--

        let handle = std::thread::spawn({
            let resume = resume_rx.clone();
            let stop_flag = stop_flag.clone();
            move || {
                //--KEYSTONE YIELD--
                let _ = wait_for_resume(&resume, &stop_flag);
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

impl ScriptProgram for KeystoneScriptProgram {
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

impl Drop for KeystoneScriptProgram {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}