use std::time::Duration;

use bevy::prelude::*;

use crate::adapter::*;
use crate::plugins::assets_loader::*;
use crate::plugins::design_resolution::ScaledViewport;
use crate::scenes::assets::DEFAULT_GROUP;

use super::components::BootRoot;
#[derive(Resource, Default)]
pub struct BootTimer {
    timer: Timer,
}

pub fn setup(
    asset_server: Res<AssetServer>,
    scaled_viewport: Res<ScaledViewport>,
    mut commands: Commands,
    mut load_writer: MessageWriter<LoadAssetGroup>,
) {
    load_writer.write(DEFAULT_GROUP);

    let fixed_width = 180.0;
    let custom_size = Vec2::new(fixed_width, fixed_width);

    commands
        .spawn((
            BootRoot,
            Sprite {
                image: asset_server.load("images/logo_with_black.png"),
                custom_size: Some(custom_size),
                ..Default::default()
            },
            Transform::default().with_scale(Vec3::splat(scaled_viewport.scale)),
        ));

    commands.insert_resource(BootTimer {
        // for testing, make it shorter
        timer: Timer::new(
            Duration::from_millis(200),
            // Duration::from_secs(30),
            TimerMode::Once,
        ),
    });
}

#[derive(Default)]
pub struct Loaded(bool);

pub fn update(
    mut reader: MessageReader<AssetGroupLoaded>,
    mut loaded: Local<Loaded>,
    mut boot_timer: ResMut<BootTimer>,
    time: Res<Time>,
    scaled_viewport: ResMut<ScaledViewport>,
    mut next_state: ResMut<NextState<GameState>>,
    mut boot_ui: Query<(&BootRoot, &mut Transform)>,
) {
    if let Ok((_, mut transform)) = boot_ui.single_mut() {
        transform.scale = Vec3::splat(scaled_viewport.scale);
        info!("Boot UI scale updated to {}", scaled_viewport.scale);
    }

    for _event in reader.read() {
        info!("Assets loaded event received");
        loaded.0 = true;
    }

    boot_timer.timer.tick(time.delta());
    if boot_timer.timer.is_finished() && loaded.0 {
        // TODO; transition to the title scene
        info!("Boot timer finished");
        next_state.set(GameState::Stage);
    }
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<BootRoot>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
