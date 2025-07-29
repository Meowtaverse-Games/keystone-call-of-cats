use bevy::prelude::*;
use keystone_cc_infra::GameState;

mod components;
mod systems;

pub struct TitlePlugin;
impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(OnEnter(GameState::Title), systems::setup_title)
            // .add_systems(Update, systems::title_input.run_if(in_state(GameState::Title)))
            .add_systems(OnExit(GameState::Title), systems::cleanup_title);
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

// fn setup_title(mut commands: Commands, asset_server: Res<AssetServer>) {
//     commands.spawn((
//         Text::new("title"),
//         TextFont {
//             font_size: 40.0,
//             font: asset_server.load("SawarabiGothic-Regular.ttf"),
//             ..default()
//         },
//         TextColor(Color::WHITE),
//     ));
// }

// fn title_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
//     if keys.just_pressed(KeyCode::Enter) {
//         next_state.set(GameState::Playing);
//     }
// }
