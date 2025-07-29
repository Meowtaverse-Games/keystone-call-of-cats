mod title;
//pub mod gameplay;
//pub mod gameover;

use bevy::prelude::*;
use title::TitlePlugin;
// use gameplay::GameplayPlugin;
// use gameover::GameOverPlugin;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TitlePlugin,
        ));
    }
}
