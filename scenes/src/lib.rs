mod boot;

use bevy::prelude::*;
use boot::BootPlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BootPlugin,));
    }
}
