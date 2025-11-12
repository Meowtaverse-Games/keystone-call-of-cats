use bevy::prelude::*;

use crate::{resources::visibility::VisibilityState, systems::engine::visibility::make_visible};

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<VisibilityState>().add_systems(
            Update,
            make_visible.run_if(in_state(VisibilityState::Hidden)),
        );
    }
}
