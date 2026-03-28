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
        mpsc::{self, Receiver, sync_channel},
    },
    thread::JoinHandle,
};

#[derive(Clone, Default)]
struct StandardApi {
    inner: Arc<Mutex<ScriptState>>,
}

impl ExternalApi for StandardApi {
    fn is_touched(&self) -> bool {
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
    fn is_empty(&self, dir: Direction) -> bool {
        let key = format!("is-empty-{}", dir_to_str(dir));
        self.inner
            .lock()
            .ok()
            .and_then(|s| s.get(&key).and_then(|v| v.as_bool()))
            .unwrap_or(false)
    }
}

impl StandardApi {
    fn write(&self, state: &ScriptState) {
        if let Ok(mut inner) = self.inner.try_lock() {
            *inner = state.clone();
        }
    }
}

#[derive(Clone)]

pub struct KeystoneScriptExecutor {
    api: StandardApi,
}

impl KeystoneScriptExecutor {
    fn new(api: StandardApi) -> Self {
        Self { api }
    }
}

impl Default for KeystoneScriptExecutor {
    fn default() -> Self {
        Self::new(StandardApi {
            inner: Arc::new(Mutex::new(ScriptState::default())),
        })
    }
}

impl ScriptRunner for KeystoneScriptExecutor {
    #[allow(dead_code, unused)]
    fn run(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        Err(ScriptExecutionError::UnsupportedLanguage(
            "Keystone scripting is not yet implemented".to_string(),
        ))
    }
}

impl ScriptStepper for KeystoneScriptExecutor {
    fn compile_step(
        &self,
        source: &str,
        _allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        let api_dyn = Arc::new(self.api.clone()) as Arc<dyn ExternalApi + Send + Sync>;
        let res = eval(source, api_dyn);
        match res {
            Ok(iter) => {
                let max_step = 100000;
                let mut step = 0;
                let preflight = iter.clone();
                for res in preflight {
                    step += 1;
                    if max_step < step {
                        break;
                    }
                    if let Err(e) = res {
                        return Err(map_error(e));
                    }
                }
                Ok(Box::new(KeystoneScriptProgram::spawn(
                    iter,
                    self.api.clone(),
                )))
            }
            Err(err) => Err(map_error(err)),
        }
    }
}

struct KeystoneScriptProgram {
    receiver: Mutex<Receiver<Option<ScriptCommand>>>,
    stop_flag: Arc<AtomicBool>,
    api: StandardApi,
    resume_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
}

impl KeystoneScriptProgram {
    fn spawn(iter: EventIterator, api: StandardApi) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_inner = stop_flag.clone();
        let (tx, rx) = sync_channel::<Option<ScriptCommand>>(1);
        let (resume_tx, resume_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            for event in iter {
                if stop_flag_inner.load(Ordering::SeqCst) {
                    break;
                }
                if resume_rx
                    .recv_timeout(std::time::Duration::from_millis(10))
                    .is_err()
                {
                    continue;
                }
                let command = map_event(event.expect("error"));
                if tx.send(command).is_err() {
                    break;
                }
            }
            let _ = tx.send(None);
        });
        Self {
            receiver: Mutex::new(rx),
            api,
            stop_flag,
            resume_tx,
            handle: Some(handle),
        }
    }
}

impl ScriptProgram for KeystoneScriptProgram {
    fn next(&mut self, state: &ScriptState) -> Option<ScriptCommand> {
        if self.stop_flag.load(Ordering::SeqCst) {
            return None;
        }
        let api_clone = self.api.clone();
        let state_clone = state.clone();
        std::thread::spawn(move || {
            api_clone.write(&state_clone);
        });
        let _ = self.resume_tx.send(());
        if let Ok(rx) = self.receiver.try_lock() {
            rx.try_recv().ok().flatten()
        } else {
            None
        }
    }
}

impl Drop for KeystoneScriptProgram {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Ok(mut rx_lock) = self.receiver.lock() {
            let _ = std::mem::replace(&mut *rx_lock, std::sync::mpsc::channel().1);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn map_event(event: Event) -> Option<ScriptCommand> {
    match event {
        Event::Move(dir) => Some(ScriptCommand::Move(map_direction(dir)?)),
        Event::Sleep(duration) => Some(ScriptCommand::Sleep(duration)),
        Event::Dig(dir) => Some(ScriptCommand::Dig(map_direction(dir)?)),
        _ => None,
    }
}

fn map_direction(dir: Direction) -> Option<MoveDirection> {
    match dir {
        Direction::Up => Some(MoveDirection::Top),
        Direction::Down => Some(MoveDirection::Down),
        Direction::Left => Some(MoveDirection::Left),
        Direction::Right => Some(MoveDirection::Right),
        _ => None,
    }
}

fn map_error(err: Error) -> ScriptExecutionError {
    match err {
        Error::InvalidOperandType { op, typ } => ScriptExecutionError::Engine(format!(
            "Cannot use type {} with operator '{}'",
            type_to_str(typ),
            op_to_str(op)
        )),
        Error::InvalidUnaryOperandType { op, typ } => ScriptExecutionError::Engine(format!(
            "Cannot use type {} with operator '{}'",
            type_to_str(typ),
            uop_to_str(op)
        )),
        Error::MismatchedTypes { op, left, right } => ScriptExecutionError::Engine(format!(
            "{} and {} cannot be used together with operator '{}'",
            type_to_str(left),
            type_to_str(right),
            op_to_str(op)
        )),
        Error::NameError { name } => {
            ScriptExecutionError::Engine(format!("Name '{}' is not defined.", name))
        }
        Error::SyntaxError { messages } => {
            ScriptExecutionError::Engine(format!("Syntax error occurred. {}", messages[0]))
        }
        Error::TooLargeNumber => ScriptExecutionError::Engine("Too large Number used.".to_string()),
        Error::UnexpectedType {
            statement,
            found_type,
        } => ScriptExecutionError::Engine(format!(
            "Unexpected type '{}' in statement '{}'",
            type_to_str(found_type),
            statement
        )),
        Error::ZeroDivisionError => {
            ScriptExecutionError::Engine("Cannot divide by zero.".to_string())
        }
        Error::ArgError {
            called,
            expected,
            got,
        } => ScriptExecutionError::Engine(format!(
            "{} expected {} args, but got {}",
            called, expected, got
        )),
    }
}

fn type_to_str(typ: Type) -> String {
    match typ {
        Type::Uint => "Uint".to_string(),
        Type::Float => "Float".to_string(),
        Type::String => "String".to_string(),
        Type::Boolean => "Boolean".to_string(),
        Type::Direction => "Direction".to_string(),
        Type::Side => "Side".to_string(),
    }
}

fn op_to_str(op: Op) -> String {
    match op {
        Op::Add => "+".to_string(),
        Op::Sub => "-".to_string(),
        Op::Mul => "*".to_string(),
        Op::Div => "/".to_string(),
        Op::And => "and".to_string(),
        Op::Or => "or".to_string(),
        Op::Eq => "==".to_string(),
        Op::Ge => ">=".to_string(),
        Op::Gt => ">".to_string(),
        Op::Le => "<=".to_string(),
        Op::Lt => "<".to_string(),
        Op::Neq => "!=".to_string(),
    }
}

fn uop_to_str(op: UnaryOp) -> String {
    match op {
        UnaryOp::Not => "not".to_string(),
    }
}

fn dir_to_str(dir: Direction) -> String {
    match dir {
        Direction::Up => String::from("top"),
        Direction::Down => String::from("down"),
        Direction::Left => String::from("left"),
        Direction::Right => String::from("right"),
        _ => String::from("unknown"),
    }
}
