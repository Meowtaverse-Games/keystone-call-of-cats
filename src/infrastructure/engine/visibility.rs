use bevy::diagnostic::FrameCount;
use bevy::prelude::*;

/// ウィンドウの最初の可視化タイミングを管理するステート
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum VisibilityState {
    #[default]
    Hidden,
    Shown,
}

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<VisibilityState>().add_systems(
            Update,
            make_visible.run_if(in_state(VisibilityState::Hidden)),
        );
    }
}

fn make_visible(
    mut window: Query<&mut Window>,
    frames: Res<FrameCount>,
    mut next_state: ResMut<NextState<VisibilityState>>,
) {
    if frames.0 == 3 {
        window.single_mut().unwrap().visible = true;
        next_state.set(VisibilityState::Shown);
    }
}
