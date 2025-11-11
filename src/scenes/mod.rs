use bevy::prelude::*;

pub mod assets;
mod boot;
mod select_stage;
pub mod stage;

use boot::BootPlugin;
use select_stage::StageSelectPlugin;
use stage::StageScenePlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BootPlugin, StageScenePlugin, StageSelectPlugin));
    }
}
