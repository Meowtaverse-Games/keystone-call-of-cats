use bevy::prelude::*;

pub mod assets;
mod boot;
mod stage;
mod title;

use boot::BootPlugin;
use stage::StagePlugin;
use title::TitlePlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BootPlugin, TitlePlugin, StagePlugin));
    }
}
