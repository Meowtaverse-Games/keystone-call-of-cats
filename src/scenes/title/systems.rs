use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_fluent::prelude::Localization;

use super::components::TitleUI;
use crate::{
    resources::{
        asset_store::AssetStore,
        design_resolution::LetterboxOffsets,
        game_state::GameState,
        launch_profile::LaunchProfile,
    },
    scenes::assets::FontKey,
    util::localization::tr,
};

pub fn setup(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
    launch_profile: Res<LaunchProfile>,
    asset_store: Res<AssetStore>,
    mut next_state: ResMut<NextState<GameState>>,
    localization: Res<Localization>,
) {
    if launch_profile.skip_title {
        info!("Skipping title scene");
        next_state.set(GameState::SelectStage);
        return;
    }

    clear_color.0 = Color::WHITE;
    letterbox_offsets.left = 0.0;
    letterbox_offsets.right = 0.0;

    let ui_root = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    commands.entity(ui_root).with_children(|parent| {
        parent
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(290.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|center| {
                center.spawn(Node { ..default() }).with_children(|stack| {
                    let title_text = tr(&*localization, "game-title-full");
                    stack.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            top: Val::Px(2.0),
                            ..default()
                        },
                        Text::new(title_text.clone()),
                        TextFont {
                            font: asset_store.font(FontKey::Title).unwrap(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.333, 0.0)),
                        TitleUI,
                    ));

                    stack.spawn((
                        // ZIndex::Local(1),
                        Text::new(title_text),
                        TextFont {
                            font: asset_store.font(FontKey::Title).unwrap(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::from(ORANGE)),
                        TitleUI,
                    ));
                });
            });
    });
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<TitleUI>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
