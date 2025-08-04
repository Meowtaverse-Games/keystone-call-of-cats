use super::components::TitleUI;
use bevy::prelude::*;

pub fn setup(
    mut clear_color: ResMut<ClearColor>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let logo_handle: Handle<Image> = asset_server.load("images/logo_with_black.png");

    clear_color.0 = Color::WHITE;

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
        TitleUI,
    ));
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<TitleUI>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
