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
    EmptyScript,
    #[allow(dead_code)]
    InvalidCommand(String),
    Engine(String),
    UnsupportedLanguage(String),
}

impl fmt::Display for ScriptExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptExecutionError::EmptyScript => write!(f, "スクリプトが空です。"),
            ScriptExecutionError::InvalidCommand(msg) => {
                write!(f, "スクリプト命令が不正です: {msg}")
            }
            ScriptExecutionError::Engine(msg) => write!(f, "スクリプト実行エラー: {msg}"),
            ScriptExecutionError::UnsupportedLanguage(lang) => {
                write!(f, "サポートされていないスクリプト言語です: {lang}")
            }
        }
    }
}

/// Abstraction for executing player-authored scripts.
pub trait ScriptRunner: Send + Sync + 'static {
    fn run(&self, source: &str) -> Result<Vec<ScriptCommand>, ScriptExecutionError>;
}
