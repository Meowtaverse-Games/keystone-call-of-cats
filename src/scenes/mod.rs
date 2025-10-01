use bevy::prelude::*;

pub mod assets;
mod boot;
mod title;
mod stage;

use boot::BootPlugin;
use title::TitlePlugin;
use stage::StagePlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BootPlugin, TitlePlugin, StagePlugin));
    }
}
