use bevy::color::palettes::css::*;
use bevy::prelude::*;

use bevy_egui::{
    egui, EguiContexts,
};

use super::components::StageUI;
use crate::plugins::{UIRoot, assets_loader::AssetStore};
use crate::scenes::assets::FontKey;


pub fn ui(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let left = egui::SidePanel::left("left")
        .resizable(true)
        .default_width(200.0)
        .min_width(100.0)
        .max_width(300.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(40, 40, 60),
            inner_margin: egui::Margin::same(10),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 150)),
            ..Default::default()
        });
    left.show(ctx, |ui| {
        ui.label("Loading...");

        ui.separator();
    });
}

pub fn setup(
    mut commands: Commands,
    ui_root: Res<UIRoot>,
    asset_store: Res<AssetStore>,
    mut clear_color: ResMut<ClearColor>,
) {
    clear_color.0 = Color::WHITE;

    commands.entity(ui_root.0).with_children(|parent| {
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
                    stack.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            top: Val::Px(2.0),
                            ..default()
                        },
                        Text::new("KEYSTONE: CALL OF CATS"),
                        TextFont {
                            font: asset_store.font(FontKey::Title).unwrap(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.333, 0.0)),
                        StageUI,
                    ));

                    stack.spawn((
                        // ZIndex::Local(1),
                        Text::new("KEYSTONE: CALL OF CATS"),
                        TextFont {
                            font: asset_store.font(FontKey::Title).unwrap(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::from(ORANGE)),
                        StageUI,
                    ));
                });
            });
    });
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<StageUI>>) {
    for ent in query.iter() {
        commands.entity(ent).despawn();
    }
}
