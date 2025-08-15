use bevy::prelude::*;

use super::components::TitleUI;
use keystone_cc_plugins::UIRoot;

pub fn setup(
    mut commands: Commands,
    ui_root: Res<UIRoot>,
    asset_server: Res<AssetServer>,
    mut clear_color: ResMut<ClearColor>,
) {
    clear_color.0 = Color::WHITE;

    commands.entity(ui_root.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    top: Val::Percent(10.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                TitleUI,
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("keystone"),
                    TextFont {
                        font: asset_server.load("fonts/PixelMplus12-Regular.ttf"),
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(Color::BLACK),
                ));
            });
    });
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<TitleUI>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
