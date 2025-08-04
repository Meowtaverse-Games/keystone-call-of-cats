use std::time::Duration;

use bevy::prelude::*;

use keystone_cc_adapter::assets_loader::AssetsLoadedEvent;
use keystone_cc_adapter::*;

use super::components::BootUI;
#[derive(Resource, Default)]
pub struct BootTimer {
    timer: Timer,
}

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let logo_handle: Handle<Image> = asset_server.load("images/logo_with_black.png");

    let fixed_width = 180.0;
    let aspect = {
        let w = 250.0;
        let h = 250.0;
        h / w
    };
    let custom_size = Vec2::new(fixed_width, fixed_width * aspect);

    commands.spawn((
        Sprite {
            image: logo_handle.clone(),
            custom_size: Some(custom_size),
            ..Default::default()
        },
        BootUI,
    ));

    commands.insert_resource(BootTimer {
        timer: Timer::new(Duration::from_secs(3), TimerMode::Once),
    });
}

#[derive(Default)]
pub struct Loaded(bool);

pub fn update(
    mut reader: EventReader<AssetsLoadedEvent>,
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
    if boot_timer.timer.finished() {
        info!("Boot timer finished");
        next_state.set(GameState::Title);
    }

    info!("Update system running in Boot state");
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<BootUI>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
