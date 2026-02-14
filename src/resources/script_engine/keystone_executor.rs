use crate::util::script_types::{
    MoveDirection, ScriptCommand, ScriptExecutionError, ScriptProgram,
    ScriptRunner, ScriptState, ScriptStepper,
};
use keystone_lang::*;
use std::{collections::HashSet,sync::Arc};

struct StandardApi;

impl ExternalApi for StandardApi {
    fn is_touched(&self) -> bool { false }
    fn is_empty(&self) -> bool { true }
}


pub struct KeystoneScriptExecutor{
    api: Arc<dyn ExternalApi + Send + Sync>
}

impl KeystoneScriptExecutor {
    pub fn new(api: Arc<dyn ExternalApi + Send + Sync>) -> Self {
        Self { api }
    }
}

impl Default for KeystoneScriptExecutor {
    fn default() -> Self {
        Self::new(Arc::new(StandardApi))
    }
}

impl ScriptRunner for KeystoneScriptExecutor {
    #[allow(dead_code, unused)]
    fn run(
        &self,
        source: &str,
        allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Vec<ScriptCommand>, ScriptExecutionError> {
        Err(ScriptExecutionError::UnsupportedLanguage("Keystone scripting is not yet implemented".to_string()))
    }
}

impl ScriptStepper for KeystoneScriptExecutor {
    fn compile_step(
        &self,
        source: &str,
        _allowed_commands: Option<&HashSet<String>>,
    ) -> Result<Box<dyn ScriptProgram>, ScriptExecutionError> {
        let res = eval(source,Arc::clone(&self.api));
        match res {
            Ok(iter) => {
                let max_step = 1000000;
                let mut step = 0;
                let preflight = iter.clone();
                for res in preflight {
                    step+=1;
                    if max_step < step{ break; }
                    if let Err(e) = res {
                        return Err(map_error(e));
                    }
                }
                Ok(Box::new(KeystoneScriptProgram { iterator: iter }))
            },
            Err(err) => {
                Err(map_error(err))
            }
        }
    }
}

struct KeystoneScriptProgram {
    iterator: EventIterator,
}

impl ScriptProgram for KeystoneScriptProgram {
    fn next(&mut self, state: &ScriptState) -> Option<ScriptCommand> {
        self.iterator.next().and_then(|event| {
            //Keystone Lang Specification Change Needed
            map_event(event.expect("error"), state)
        })
    }
}

fn map_event(event: Event, _state: &ScriptState) -> Option<ScriptCommand> {
    match event {
        Event::Move(dir) => Some(ScriptCommand::Move(map_direction(dir)?)),
        Event::Sleep(duration) => Some(ScriptCommand::Sleep(duration)),
        Event::Dig(dir) => Some(ScriptCommand::Dig(map_direction(dir)?)),
        _ => None,
    }
}

fn map_direction(dir: Direction) -> Option<MoveDirection>{
    match dir {
        Direction::Up => Some(MoveDirection::Top),
        Direction::Down => Some(MoveDirection::Down),
        Direction::Left => Some(MoveDirection::Left),
        Direction::Right => Some(MoveDirection::Right),
        _ => None
    }
}


fn map_error(err:Error)->ScriptExecutionError{
    match err {
        Error::InvalidOperandType { op,typ } => 
            ScriptExecutionError::Engine(format!("Cannot use type {} with operator '{}'",type_to_str(typ),op_to_str(op))),
        Error::InvalidUnaryOperandType { op, typ } => 
            ScriptExecutionError::Engine(format!("Cannot use type {} with operator '{}'",type_to_str(typ),uop_to_str(op))),
        Error::MismatchedTypes { op, left, right } => 
            ScriptExecutionError::Engine(format!("{} and {} cannot be used together with operator '{}'",type_to_str(left),type_to_str(right),op_to_str(op))),
        Error::NameError { name } => 
            ScriptExecutionError::Engine(format!("Name '{}' is not defined.", name)),
        Error::SyntaxError { messages } => 
            ScriptExecutionError::Engine(format!("Syntax error occurred. {}", messages[0])),
        Error::TooLargeNumber => 
            ScriptExecutionError::Engine(format!("Too large Number used.")),
        Error::UnexpectedType { statement, found_type } => 
            ScriptExecutionError::Engine(format!("Unexpected type '{}' in statement '{}'", type_to_str(found_type), statement)),
        Error::ZeroDivisionError => 
            ScriptExecutionError::Engine(format!("Cannot divide by zero.")),
        Error::ArgError { called, expected, got } => 
            ScriptExecutionError::Engine(format!("{} expected {} args, but got {}",called,expected,got)),
    }
}

fn type_to_str(typ:Type) -> String{
    match typ{
        Type::Uint => "Uint".to_string(),
        Type::Float => "Float".to_string(),
        Type::String => "String".to_string(),
        Type::Boolean => "Boolean".to_string(),
        Type::Direction => "Direction".to_string(),
        Type::Side => "Side".to_string(),
    }
}

fn op_to_str(op: Op) -> String{
    match op{
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

fn uop_to_str(op:UnaryOp) -> String{
    match op{
        UnaryOp::Not => "not".to_string()
    }
}