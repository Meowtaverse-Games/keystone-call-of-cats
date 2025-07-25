use bevy::prelude::*;
use bevy::diagnostic::FrameCount;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum VisibilityState {
    #[default] Hidden,
    Shown,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {primary_window: Some(Window {
                visible: false,
                ..default()
            }),
            ..default()
            }))
        .init_state::<VisibilityState>()
        .add_plugins(boundary_plugin::BoundaryPlugin)
        .add_systems(Update, make_visible.run_if(in_state(VisibilityState::Hidden)))
        .run();
}

fn make_visible(
    mut window: Query<&mut Window>,
    frames: Res<FrameCount>,
    mut next_state: ResMut<NextState<VisibilityState>>,
) {
    if frames.0 == 3 {
        window.single_mut().unwrap().visible = true;
        next_state.set(VisibilityState::Shown)
    }
}

mod boundary_plugin {
    use bevy::prelude::*;
    use keystone_cc_infra::*;
    use keystone_cc_core::boundary::ScoreRepo;

    pub struct BoundaryPlugin;

    impl Plugin for BoundaryPlugin {
        fn build(&self, app: &mut App) {
            app
                .init_resource::<Events<BevyGameEvent>>()
                .add_event::<BevyGameEvent>()
                .insert_resource(FileScoreRepo)
                .add_systems(Startup, setup_score_repo);
        }
    }

    fn setup_score_repo() {
        let repo = FileScoreRepo;
        repo.save(42);
    }
}
