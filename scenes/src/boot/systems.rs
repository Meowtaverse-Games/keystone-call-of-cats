use std::time::Duration;

use bevy::prelude::*;

use keystone_cc_adapter::*;
use keystone_cc_plugins::assets_loader::*;

use crate::assets::DEFAULT_GROUP;

use super::components::BootUI;
#[derive(Resource, Default)]
pub struct BootTimer {
    timer: Timer,
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut load_writer: EventWriter<LoadAssetGroup>,
) {
    load_writer.write(DEFAULT_GROUP);

    let fixed_width = 180.0;
    let custom_size = Vec2::new(fixed_width, fixed_width);

    commands
        .spawn((Node { ..default() }, BootUI))
        .with_children(|p| {
            p.spawn(Sprite {
                image: asset_server.load("images/logo_with_black.png"),
                custom_size: Some(custom_size),
                ..Default::default()
            });
        });

    commands.insert_resource(BootTimer {
        timer: Timer::new(Duration::from_secs(3), TimerMode::Once),
    });
}

#[derive(Default)]
pub struct Loaded(bool);

pub fn update(
    mut reader: EventReader<AssetGroupLoaded>,
    mut loaded: Local<Loaded>,
    mut boot_timer: ResMut<BootTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for _event in reader.read() {
        info!("Assets loaded event received");
        loaded.0 = true;
    }

    boot_timer.timer.tick(time.delta());
    if boot_timer.timer.finished() && loaded.0 {
        info!("Boot timer finished");
        next_state.set(GameState::Title);
    }
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<BootUI>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
