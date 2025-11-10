use bevy::prelude::*;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Boot,
    SelectStage,
    Stage,
}

pub const TOTAL_STAGE_SLOTS: usize = 20;
