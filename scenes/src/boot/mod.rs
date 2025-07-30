use bevy::prelude::*;
use keystone_cc_infra::GameState;

mod components;
mod systems;

pub struct BootPlugin;
impl Plugin for BootPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(OnEnter(GameState::Boot), systems::setup)
            .add_systems(OnExit(GameState::Boot), systems::cleanup);
    }
}

mod boundary_plugin {
    use bevy::prelude::*;
    use keystone_cc_core::boundary::ScoreRepo;
    use keystone_cc_infra::*;

    pub struct BoundaryPlugin;

    impl Plugin for BoundaryPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<Events<BevyGameEvent>>()
                .add_event::<BevyGameEvent>()
                .insert_resource(FileScoreRepo)
                .add_systems(Startup, setup_score_repo)
                .add_systems(Update, load_score);
        }
    }

    fn setup_score_repo(repo: Res<FileScoreRepo>) {
        repo.save(42);
    }

    fn load_score(repo: Res<FileScoreRepo>) {
        println!("{}", repo.load());
    }
}
