use bevy::prelude::*;

pub mod assets;
pub mod audio;

mod boot;
use boot::BootPlugin;

mod select_stage;
use select_stage::StageSelectPlugin;

pub mod stage;
use audio::UiAudioPlugin;
use stage::StageScenePlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            UiAudioPlugin,
            BootPlugin,
            StageSelectPlugin,
            StageScenePlugin,
        ));
    }
}
