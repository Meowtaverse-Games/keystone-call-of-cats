mod boot;
mod title;

use bevy::prelude::*;

use boot::BootPlugin;
use title::TitlePlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BootPlugin, TitlePlugin));
    }
}
