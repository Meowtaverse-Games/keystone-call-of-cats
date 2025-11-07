use std::path::Path;

use bevy::prelude::*;
use bevy_ecs::hierarchy::ChildSpawnerCommands;

use super::components::*;
use crate::{
    adapter::GameState,
    plugins::{
        TiledMapAssets, TiledMapLibrary, assets_loader::AssetStore,
        design_resolution::LetterboxOffsets,
    },
    scenes::{assets::FontKey, stage::StageProgression},
};

const TOTAL_STAGE_SLOTS: usize = 20;
const CARDS_PER_PAGE: usize = 3;
const CARD_WIDTH: f32 = 360.0;
const CARD_HEIGHT: f32 = 320.0;
const CARD_GAP: f32 = 32.0;
const SECTION_SPACING: f32 = 20.0;

#[derive(Resource)]
pub struct StageSelectState {
    current_page: usize,
    cards_per_page: usize,
    total_entries: usize,
}

impl StageSelectState {
    pub fn new(total_entries: usize, cards_per_page: usize) -> Self {
        Self {
            current_page: 0,
            cards_per_page: cards_per_page.max(1),
            total_entries: total_entries.max(1),
        }
    }

    pub fn total_pages(&self) -> usize {
        ((self.total_entries + self.cards_per_page - 1) / self.cards_per_page).max(1)
    }

    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let start = (self.current_page * self.cards_per_page).min(self.total_entries);
        let end = (start + self.cards_per_page).min(self.total_entries);
        start..end
    }

    pub fn move_page(&mut self, delta: isize) -> bool {
        let total = self.total_pages() as isize;
        if total <= 0 {
            return false;
        }

        let mut next = self.current_page as isize + delta;
        next = next.clamp(0, total - 1);

        let changed = next as usize != self.current_page;
        if changed {
            self.current_page = next as usize;
        }
        changed
    }
}

struct StageEntry {
    index: usize,
    title: String,
    playable: bool,
}

impl StageEntry {
    fn playable(index: usize, map: &TiledMapAssets) -> Self {
        Self {
            index,
            title: display_name(map),
            playable: true,
        }
    }

    fn locked(index: usize) -> Self {
        Self {
            index,
            title: format!("STAGE {:02}", index + 1),
            playable: false,
        }
    }
}

pub fn setup(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
    asset_store: Res<AssetStore>,
    tiled_maps: Res<TiledMapLibrary>,
) {
    clear_color.0 = background_color();
    letterbox_offsets.left = 0.0;
    letterbox_offsets.right = 0.0;

    let font = if let Some(handle) = asset_store.font(FontKey::Default) {
        handle
    } else {
        warn!("StageSelect: default font is not available, UI text may be missing");
        Handle::default()
    };

    let entries = build_stage_entries(&tiled_maps);
    let state = StageSelectState::new(entries.len(), CARDS_PER_PAGE);
    let page_text = format!("{}/{}", state.current_page + 1, state.total_pages());
    commands.insert_resource(state);

    let root = commands
        .spawn((
            StageSelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Stretch,
                padding: UiRect::axes(Val::Px(48.0), Val::Px(32.0)),
                row_gap: Val::Px(SECTION_SPACING),
                ..default()
            },
            BackgroundColor(background_color()),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        spawn_top_bar(parent, &font);
        spawn_stage_cards(parent, &entries, &font);
        spawn_bottom_bar(parent, &font, &page_text);
    });
}

pub fn cleanup(mut commands: Commands, roots: Query<Entity, With<StageSelectRoot>>) {
    for entity in roots.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<StageSelectState>();
}

pub fn handle_back_button(
    mut interactions: Query<(&StageBackButton, &Interaction), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (_, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Title);
        }
    }
}

pub fn handle_nav_buttons(
    mut interactions: Query<(&StagePageButton, &Interaction), Changed<Interaction>>,
    mut state: ResMut<StageSelectState>,
) {
    for (button, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            state.move_page(button.delta);
        }
    }
}

pub fn handle_play_buttons(
    mut interactions: Query<(&StagePlayButton, &Interaction), Changed<Interaction>>,
    tiled_maps: Res<TiledMapLibrary>,
    mut progression: ResMut<StageProgression>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (button, interaction) in &mut interactions {
        if !button.enabled {
            continue;
        }

        if *interaction == Interaction::Pressed {
            if progression.select(button.stage_index, tiled_maps.as_ref()) {
                next_state.set(GameState::Stage);
            } else {
                warn!(
                    "Stage {} is not available in the Tiled map library",
                    button.stage_index + 1
                );
            }
        }
    }
}

pub fn handle_keyboard_navigation(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<StageSelectState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::ArrowRight) {
        state.move_page(1);
    }

    if keys.just_pressed(KeyCode::ArrowLeft) {
        state.move_page(-1);
    }

    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Title);
    }
}

pub fn refresh_cards(state: Res<StageSelectState>, mut cards: Query<(&StageCard, &mut Node)>) {
    if !state.is_changed() {
        return;
    }

    let visible_range = state.visible_range();

    for (card, mut node) in &mut cards {
        node.display = if visible_range.contains(&card.index) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub fn update_page_indicator(
    state: Res<StageSelectState>,
    mut indicators: Query<&mut Text, With<StagePageIndicator>>,
) {
    if !state.is_changed() {
        return;
    }

    let total_pages = state.total_pages();
    let value = format!("{}/{}", state.current_page + 1, total_pages);

    for mut text in &mut indicators {
        text.0 = value.clone();
    }
}

pub fn update_button_visuals(
    mut query: Query<(&ButtonVisual, &Interaction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (visual, interaction, mut color) in &mut query {
        color.0 = visual.color_for(interaction);
    }
}

fn spawn_top_bar(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|bar| {
            spawn_back_button(bar, font);
            spawn_category_label(bar, font, "DEFAULT");
            spawn_category_label(bar, font, "CUSTOM");
        });
}

fn spawn_back_button(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    let visual = ButtonVisual::new(
        subtle_button_color(0.0),
        subtle_button_color(0.15),
        subtle_button_color(0.3),
        subtle_button_color(0.0),
        true,
    );
    let initial = button_initial_color(&visual);

    parent
        .spawn((
            StageBackButton,
            Button,
            visual,
            Node {
                padding: UiRect::all(Val::Px(18.0)),
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BorderColor::all(primary_text_color()),
            BackgroundColor(initial),
        ))
        .with_children(|btn| {
            btn.spawn(Text::new("BACK"))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 42.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_category_label(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BorderColor::all(primary_text_color()),
        ))
        .with_children(|node| {
            node.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 38.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_stage_cards(
    parent: &mut ChildSpawnerCommands,
    entries: &[StageEntry],
    font: &Handle<Font>,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(CARD_GAP),
            flex_wrap: FlexWrap::NoWrap,
            ..default()
        })
        .with_children(|row| {
            for entry in entries {
                spawn_stage_card(row, entry, font);
            }
        });
}

fn spawn_stage_card(parent: &mut ChildSpawnerCommands, entry: &StageEntry, font: &Handle<Font>) {
    let display = if entry.index < CARDS_PER_PAGE {
        Display::Flex
    } else {
        Display::None
    };

    parent
        .spawn((
            StageCard { index: entry.index },
            Node {
                width: Val::Px(CARD_WIDTH),
                height: Val::Px(CARD_HEIGHT),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                row_gap: Val::Px(16.0),
                padding: UiRect::all(Val::Px(18.0)),
                display,
                ..default()
            },
            BackgroundColor(card_background_color()),
            BorderColor::all(primary_text_color()),
        ))
        .with_children(|card| {
            card.spawn(Text::new(entry.title.clone()))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 32.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));

            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                column_gap: Val::Px(16.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|row| {
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|stats| {
                    stats
                        .spawn(Text::new("GLOBAL HIGHSCORES:"))
                        .insert(TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        })
                        .insert(TextColor(primary_text_color()));

                    stats
                        .spawn(Text::new("BEST TIME\n--"))
                        .insert(TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        })
                        .insert(TextColor(secondary_text_color()));

                    stats
                        .spawn(Text::new("BEST SCRIPT\n--"))
                        .insert(TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        })
                        .insert(TextColor(secondary_text_color()));
                });

                row.spawn((
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    BackgroundColor(preview_background()),
                ));
            });

            spawn_play_button(card, entry, font);
        });
}

fn spawn_play_button(parent: &mut ChildSpawnerCommands, entry: &StageEntry, font: &Handle<Font>) {
    let enabled = entry.playable;
    let visual = ButtonVisual::new(
        accent_color(),
        accent_hover_color(),
        accent_pressed_color(),
        disabled_accent_color(),
        enabled,
    );

    let label = if enabled { "PLAY >" } else { "LOCKED" };

    parent
        .spawn((
            StagePlayButton {
                stage_index: entry.index,
                enabled,
            },
            Button,
            visual,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                align_self: AlignSelf::FlexEnd,
                padding: UiRect::axes(Val::Px(0.0), Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(button_initial_color(&visual)),
        ))
        .with_children(|btn| {
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 28.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_bottom_bar(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, initial_value: &str) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|nav| {
            spawn_nav_button(nav, font, "<", -1);
            nav.spawn((
                StagePageIndicator,
                Text::new(initial_value),
                TextFont {
                    font: font.clone(),
                    font_size: 26.0,
                    ..default()
                },
                TextColor(primary_text_color()),
            ));
            spawn_nav_button(nav, font, ">", 1);
        });
}

fn spawn_nav_button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    delta: isize,
) {
    let visual = ButtonVisual::new(
        subtle_button_color(0.0),
        subtle_button_color(0.15),
        subtle_button_color(0.3),
        subtle_button_color(0.0),
        true,
    );

    parent
        .spawn((
            StagePageButton { delta },
            Button,
            visual,
            Node {
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(primary_text_color()),
            BackgroundColor(button_initial_color(&visual)),
        ))
        .with_children(|btn| {
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn button_initial_color(visual: &ButtonVisual) -> Color {
    if visual.enabled {
        visual.normal
    } else {
        visual.disabled
    }
}

fn build_stage_entries(library: &TiledMapLibrary) -> Vec<StageEntry> {
    let total_slots = library.len().max(TOTAL_STAGE_SLOTS).max(1);
    let mut entries = Vec::with_capacity(total_slots);

    for index in 0..total_slots {
        if let Some(map) = library.get(index) {
            entries.push(StageEntry::playable(index, map));
        } else {
            entries.push(StageEntry::locked(index));
        }
    }

    entries
}

fn display_name(map: &TiledMapAssets) -> String {
    Path::new(map.map_path())
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|name| name.replace('_', " ").to_uppercase())
        .unwrap_or_else(|| format!("STAGE {}", map.map_path()))
}

fn background_color() -> Color {
    Color::srgb(0.05, 0.07, 0.14)
}

fn card_background_color() -> Color {
    Color::srgb(0.09, 0.11, 0.2)
}

fn preview_background() -> Color {
    Color::srgb(0.13, 0.16, 0.28)
}

fn primary_text_color() -> Color {
    Color::srgb(0.95, 0.96, 0.98)
}

fn secondary_text_color() -> Color {
    Color::srgb(0.75, 0.78, 0.84)
}

fn accent_color() -> Color {
    Color::srgb(0.97, 0.32, 0.54)
}

fn accent_hover_color() -> Color {
    Color::srgb(0.98, 0.44, 0.64)
}

fn accent_pressed_color() -> Color {
    Color::srgb(0.85, 0.25, 0.46)
}

fn disabled_accent_color() -> Color {
    Color::srgb(0.3, 0.32, 0.4)
}

fn subtle_button_color(alpha: f32) -> Color {
    Color::srgba(1.0, 1.0, 1.0, alpha)
}
