use bevy::prelude::*;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(ai_system);
    }
}

fn ai_system() {
    // TODO: Implement AI logic
}
