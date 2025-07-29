use bevy::prelude::*;
use keystone_cc_infra::GameState;

pub struct TitlePlugin;
impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Title), setup_title);
        // .add_systems(Update, title_input.run_if(in_state(GameState::Title)))
        // .add_systems(OnExit(GameState::Title), cleanup_title);
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

fn main2() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(OnEnter(GameState::Title), setup_title)
        .add_systems(Update, title_input.run_if(in_state(GameState::Title)))
        .add_systems(OnExit(GameState::Title), cleanup)
        .add_systems(OnEnter(GameState::Playing), setup_gameplay)
        .add_systems(Update, gameplay_input.run_if(in_state(GameState::Playing)))
        .add_systems(OnExit(GameState::Playing), cleanup)
        .run();
}

fn setup_title(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("hogehoge");
    commands.spawn((
        Text::new("title"),
        TextFont {
            font_size: 40.0,
            font: asset_server.load("SawarabiGothic-Regular.ttf"),
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn title_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::Playing);
    }
}

// プレイ画面
fn setup_gameplay(mut commands: Commands) {
    commands.spawn((
        Text::new("ゲーム中: Gでゲームオーバー"),
        TextFont {
            font_size: 40.0,
            //color: Color::GREEN,
            font: Default::default(),
            ..default()
        },
    ));
}

fn gameplay_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::KeyG) {
        // next_state.set(GameState::GameOver);
    }
}

// ゲームオーバー画面
fn setup_gameover(mut commands: Commands) {
    commands.spawn((
        Text::new("ゲームオーバー: Escでタイトルへ"),
        TextFont {
            font_size: 40.0,
            // color: Color::RED,
            ..default()
        },
    ));
}

fn gameover_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Title);
    }
}

// エンティティの掃除
fn cleanup(mut commands: Commands, query: Query<Entity, With<Text>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
