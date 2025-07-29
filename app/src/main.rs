use bevy::prelude::*;
use keystone_cc_infra::game_state::GameState;
use keystone_cc_infra::VisibilityPlugin;
use keystone_cc_scenes::title::TitlePlugin;

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                //visible: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .init_state::<GameState>()
        .add_plugins(VisibilityPlugin)
        .add_plugins((TitlePlugin,))
        .run();
}

fn setup(mut commands: Commands) {
    // commands.spawn(MainCamera);
    commands.spawn(Camera2d::default());
}
