use std::fmt;

/// Represents a command emitted by a script.
#[derive(Debug, Clone)]
pub enum ScriptCommand {
    Move(MoveDirection),
    Sleep(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Left,
    Top,
    Right,
    Down,
}

impl MoveDirection {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "left" => Some(Self::Left),
            "top" | "up" => Some(Self::Top),
            "right" => Some(Self::Right),
            "down" => Some(Self::Down),
            _ => None,
        }
    }
}

impl fmt::Display for MoveDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            MoveDirection::Left => "left",
            MoveDirection::Top => "top",
            MoveDirection::Right => "right",
            MoveDirection::Down => "down",
        };
        write!(f, "{text}")
    }
}

/// High-level errors surfaced when running scripts.
#[derive(Debug)]
pub enum ScriptExecutionError {
    InvalidMoveDirection {
        direction: String,
    },
    InvalidSleepDuration,
    Engine(String),
    UnsupportedLanguage(String),
    #[allow(dead_code)]
    InvalidCommand(String),
}

impl fmt::Display for ScriptExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptExecutionError::EmptyScript => write!(f, "Script is empty."),
            ScriptExecutionError::InvalidMoveDirection { direction } => {
                write!(f, "Invalid move direction: {direction}")
            }
            ScriptExecutionError::InvalidSleepDuration => {
                write!(f, "sleep duration must be zero or greater.")
            }
            ScriptExecutionError::Engine(msg) => write!(f, "Script runtime error: {msg}"),
            ScriptExecutionError::UnsupportedLanguage(lang) => {
                write!(f, "Unsupported script language: {lang}")
            }
            ScriptExecutionError::InvalidCommand(msg) => {
                write!(f, "Invalid command: {msg}")
            }
        }
    }
}

/// Abstraction for executing player-authored scripts.
#[allow(dead_code)]
pub trait ScriptRunner: Send + Sync + 'static {
    fn run(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError>;
}

/// Iterator-like interface for step-by-step command generation.
/// Implementations should be cheap to `next` and honor safety limits internally.
pub trait ScriptProgram: Send + Sync + 'static {
    /// Produces the next command, or None if finished.
    fn next(&mut self) -> Option<ScriptCommand>;
}

/// Compiles a script into a step-executable program.
pub trait ScriptStepper: Send + Sync + 'static {
    fn compile_step(&self, source: &str) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError>;
}
