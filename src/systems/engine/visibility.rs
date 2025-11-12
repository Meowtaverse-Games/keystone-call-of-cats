use bevy::{diagnostic::FrameCount, prelude::*};

use crate::resources::visibility::VisibilityState;

pub fn make_visible(
    mut windows: Query<&mut Window>,
    frames: Res<FrameCount>,
    mut next_state: ResMut<NextState<VisibilityState>>,
) {
    if frames.0 == 3 {
        if let Ok(mut window) = windows.single_mut() {
            window.visible = true;
        }
        next_state.set(VisibilityState::Shown);
    }
}
