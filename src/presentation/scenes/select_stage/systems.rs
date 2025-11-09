use bevy::app::AppExit;
use bevy::prelude::MessageWriter;
use bevy::prelude::*;
use bevy::ui::BorderRadius;
use bevy_ecs::hierarchy::ChildSpawnerCommands;

use super::components::*;
use crate::{
    application::{GameState, StageProgressUseCase},
    domain::stage_progress::StageProgress,
    infrastructure::engine::*,
    presentation::scenes::{assets::FontKey, stage::StageProgression},
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
        self.total_entries.div_ceil(self.cards_per_page).max(1)
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

struct StageSummary {
    total: usize,
    unlocked: usize,
    locked: usize,
    highlight: String,
}

impl StageSummary {
    fn from_entries(entries: &[StageEntry]) -> Self {
        let total = entries.len();
        let unlocked = entries.iter().filter(|entry| entry.playable).count();
        let locked = total.saturating_sub(unlocked);
        let highlight = entries
            .iter()
            .find(|entry| entry.playable)
            .map(|entry| entry.title.clone())
            .unwrap_or_else(|| "COMING SOON".to_string());

        Self {
            total,
            unlocked,
            locked,
            highlight,
        }
    }
}

impl StageEntry {
    fn playable(index: usize) -> Self {
        Self {
            index,
            title: format!("STAGE-{}", index),
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
    steam: Res<SteamClient>,
) {
    let progress = StageProgressUseCase::new(steam.as_ref())
        .load_or_default()
        .unwrap_or_else(|err| {
            warn!("StageSelect: failed to load stage progress: {err}");
            StageProgress::default()
        });

    clear_color.0 = background_color();
    letterbox_offsets.left = 0.0;
    letterbox_offsets.right = 0.0;

    let font = if let Some(handle) = asset_store.font(FontKey::Default) {
        handle
    } else {
        warn!("StageSelect: default font is not available, UI text may be missing");
        Handle::default()
    };
    let display_font = asset_store
        .font(FontKey::Title)
        .unwrap_or_else(|| font.clone());

    let entries = build_stage_entries(&progress);
    let summary = StageSummary::from_entries(&entries);
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
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                padding: UiRect::axes(Val::Px(48.0), Val::Px(32.0)),
                row_gap: Val::Px(SECTION_SPACING * 1.25),
                ..default()
            },
            BackgroundColor(background_color()),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        spawn_glow_layers(parent);
        spawn_hero_section(parent, &font, &display_font, &summary, &page_text);
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
    mut exit_events: MessageWriter<AppExit>,
) {
    for (_, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            exit_events.write(AppExit::Success);
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
    mut progression: ResMut<StageProgression>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (button, interaction) in &mut interactions {
        if !button.enabled {
            continue;
        }

        if *interaction == Interaction::Pressed {
            if progression.select(button.stage_index) {
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
    mut exit_events: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::ArrowRight) {
        state.move_page(1);
    }

    if keys.just_pressed(KeyCode::ArrowLeft) {
        state.move_page(-1);
    }

    if keys.just_pressed(KeyCode::Escape) {
        exit_events.write(AppExit::Success);
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

fn spawn_glow_layers(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-160.0),
            top: Val::Px(32.0),
            width: Val::Px(360.0),
            height: Val::Px(360.0),
            ..default()
        },
        BorderRadius::all(Val::Px(360.0)),
        BackgroundColor(primary_glow_color()),
        ZIndex(-1),
    ));

    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(-120.0),
            bottom: Val::Px(-80.0),
            width: Val::Px(420.0),
            height: Val::Px(420.0),
            ..default()
        },
        BorderRadius::all(Val::Px(420.0)),
        BackgroundColor(secondary_glow_color()),
        ZIndex(-1),
    ));
}

fn spawn_hero_section(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    display_font: &Handle<Font>,
    summary: &StageSummary,
    page_text: &str,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            ..default()
        })
        .with_children(|hero| {
            hero.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                spawn_status_badge(row, font, "EXPERIMENTAL BUILD");
                spawn_back_button(row, font);
            });

            hero.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(32.0),
                align_items: AlignItems::Stretch,
                ..default()
            })
            .with_children(|content| {
                spawn_hero_copy(content, font, display_font, summary);
                spawn_highlight_card(content, font, summary, page_text);
            });
        });
}

fn spawn_status_badge(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(999.0)),
            BackgroundColor(badge_background_color()),
        ))
        .with_children(|badge| {
            badge
                .spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                })
                .insert(TextColor(secondary_text_color()));
        });
}

fn spawn_hero_copy(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    display_font: &Handle<Font>,
    summary: &StageSummary,
) {
    parent
        .spawn(Node {
            flex_grow: 2.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|left| {
            left.spawn(Node {
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|stack| {
                stack.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(4.0),
                        ..default()
                    },
                    Text::new("KEYSTONE · CALL OF CATS"),
                    TextFont {
                        font: display_font.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(hero_shadow_color()),
                ));

                stack.spawn((
                    Text::new("KEYSTONE · CALL OF CATS"),
                    TextFont {
                        font: display_font.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(hero_title_color()),
                ));
            });

            left.spawn(Text::new(
                "Drop into a vibrant kitty multiverse, remix your best scripts,\
                 \nand pursue the sharpest keystones.",
            ))
            .insert(TextFont {
                font: font.clone(),
                font_size: 24.0,
                ..default()
            })
            .insert(TextColor(secondary_text_color()));

            left.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|stats| {
                spawn_stat_card(
                    stats,
                    font,
                    "UNLOCKED",
                    &format!("{}", summary.unlocked),
                    true,
                );
                spawn_stat_card(stats, font, "LOCKED", &format!("{}", summary.locked), false);
                spawn_stat_card(stats, font, "SLOTS", &format!("{}", summary.total), false);
            });
        });
}

fn spawn_stat_card(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    value: &str,
    accent: bool,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(20.0), Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BorderRadius::all(Val::Px(18.0)),
            BackgroundColor(stat_card_background(accent)),
        ))
        .with_children(|card| {
            card.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                })
                .insert(TextColor(secondary_text_color()));

            card.spawn(Text::new(value))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 32.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_highlight_card(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    summary: &StageSummary,
    page_text: &str,
) {
    parent
        .spawn((
            Node {
                flex_grow: 1.0,
                padding: UiRect::all(Val::Px(28.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                align_self: AlignSelf::Stretch,
                ..default()
            },
            BorderRadius::all(Val::Px(32.0)),
            BackgroundColor(hero_card_background()),
        ))
        .with_children(|card| {
            card.spawn(Text::new("FEATURED STAGE"))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                })
                .insert(TextColor(secondary_text_color()));

            card.spawn(Text::new(summary.highlight.clone()))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 34.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));

            card.spawn(Text::new(
                "Dial in your keystone strategy before the cats do.",
            ))
            .insert(TextFont {
                font: font.clone(),
                font_size: 20.0,
                ..default()
            })
            .insert(TextColor(secondary_text_color()));

            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|chips| {
                spawn_highlight_chip(chips, font, &format!("PAGES {}", page_text));
                spawn_highlight_chip(chips, font, "STORY MODE");
            });
        });
}

fn spawn_highlight_chip(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, label: &str) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(999.0)),
            BackgroundColor(badge_background_color()),
        ))
        .with_children(|chip| {
            chip.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_back_button(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    let visual = ButtonVisual::new(
        hero_button_color(),
        hero_button_hover_color(),
        hero_button_pressed_color(),
        subtle_button_color(0.2),
        true,
    );
    let initial = button_initial_color(&visual);

    parent
        .spawn((
            StageBackButton,
            Button,
            visual,
            Node {
                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(999.0)),
            BackgroundColor(initial),
        ))
        .with_children(|btn| {
            btn.spawn(Text::new("EXIT"))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 28.0,
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
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(CARD_GAP),
            column_gap: Val::Px(CARD_GAP),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|grid| {
            for entry in entries {
                grid.spawn((
                    StageCard { index: entry.index },
                    Node {
                        width: Val::Px(CARD_WIDTH),
                        min_height: Val::Px(CARD_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        display: if entry.index < CARDS_PER_PAGE {
                            Display::Flex
                        } else {
                            Display::None
                        },
                        ..default()
                    },
                    BorderRadius::all(Val::Px(28.0)),
                    BackgroundColor(card_background_color()),
                    BorderColor::all(card_border_color()),
                ))
                .with_children(|card| {
                    card.spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|header| {
                        header
                            .spawn(Text::new(format!("STAGE {:02}", entry.index + 1)))
                            .insert(TextFont {
                                font: font.clone(),
                                font_size: 18.0,
                                ..default()
                            })
                            .insert(TextColor(secondary_text_color()));

                        spawn_stage_chip(header, font, entry.playable);
                    });

                    card.spawn(Text::new(entry.title.clone()))
                        .insert(TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            ..default()
                        })
                        .insert(TextColor(primary_text_color()));

                    card.spawn(Text::new(if entry.playable {
                        "Sprint-ready layout for confident coders."
                    } else {
                        "Reach the keystone above to unlock this remix."
                    }))
                    .insert(TextFont {
                        font: font.clone(),
                        font_size: 18.0,
                        ..default()
                    })
                    .insert(TextColor(secondary_text_color()));

                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|stats| {
                        spawn_mini_stat(stats, font, "BEST TIME", "--");
                        spawn_mini_stat(stats, font, "BEST SCRIPT", "--");
                    });

                    card.spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        BorderRadius::all(Val::Px(20.0)),
                        BackgroundColor(preview_background()),
                    ))
                    .with_children(|_| {});

                    spawn_play_button(card, entry, font);
                });
            }
        });
}

fn spawn_stage_chip(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, playable: bool) {
    let (label, color) = if playable {
        ("READY", hero_button_color())
    } else {
        ("LOCKED", disabled_accent_color())
    };

    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(999.0)),
            BackgroundColor(color),
        ))
        .with_children(|chip| {
            chip.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_mini_stat(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    value: &str,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|stat| {
            stat.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                })
                .insert(TextColor(secondary_text_color()));

            stat.spawn(Text::new(value))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
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
                padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(999.0)),
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

fn spawn_bottom_bar(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, initial_value: &str) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(20.0),
                padding: UiRect::axes(Val::Px(28.0), Val::Px(18.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(28.0)),
            BackgroundColor(nav_background_color()),
            BorderColor::all(card_border_color()),
        ))
        .with_children(|nav| {
            spawn_nav_button(nav, font, "<", -1);
            nav.spawn((
                StagePageIndicator,
                Text::new(initial_value),
                TextFont {
                    font: font.clone(),
                    font_size: 28.0,
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
        subtle_button_color(0.2),
        subtle_button_color(0.35),
        subtle_button_color(0.5),
        subtle_button_color(0.08),
        true,
    );

    parent
        .spawn((
            StagePageButton { delta },
            Button,
            visual,
            Node {
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(18.0)),
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

fn build_stage_entries(progress: &StageProgress) -> Vec<StageEntry> {
    let mut entries = Vec::with_capacity(TOTAL_STAGE_SLOTS);

    for index in 0..TOTAL_STAGE_SLOTS {
        if progress.is_unlocked(index) {
            entries.push(StageEntry {
                index,
                title: format!("STAGE {:02}", index + 1),
                playable: true,
            });
        } else {
            entries.push(StageEntry::locked(index));
        }
    }

    entries
}

fn background_color() -> Color {
    Color::srgb(0.02, 0.03, 0.07)
}

fn primary_glow_color() -> Color {
    Color::srgba(0.99, 0.35, 0.71, 0.22)
}

fn secondary_glow_color() -> Color {
    Color::srgba(0.28, 0.63, 0.98, 0.18)
}

fn card_background_color() -> Color {
    Color::srgb(0.09, 0.11, 0.2)
}

fn card_border_color() -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.08)
}

fn preview_background() -> Color {
    Color::srgb(0.14, 0.17, 0.31)
}

fn hero_card_background() -> Color {
    Color::srgb(0.11, 0.14, 0.28)
}

fn nav_background_color() -> Color {
    Color::srgba(0.1, 0.13, 0.23, 0.85)
}

fn primary_text_color() -> Color {
    Color::srgb(0.96, 0.97, 1.0)
}

fn secondary_text_color() -> Color {
    Color::srgb(0.73, 0.78, 0.9)
}

fn badge_background_color() -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.08)
}

fn hero_title_color() -> Color {
    Color::srgb(0.99, 0.56, 0.79)
}

fn hero_shadow_color() -> Color {
    Color::srgb(0.1, 0.03, 0.19)
}

fn stat_card_background(accent: bool) -> Color {
    if accent {
        Color::srgba(0.99, 0.38, 0.67, 0.28)
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.05)
    }
}

fn hero_button_color() -> Color {
    Color::srgb(0.96, 0.34, 0.53)
}

fn hero_button_hover_color() -> Color {
    Color::srgb(0.98, 0.42, 0.63)
}

fn hero_button_pressed_color() -> Color {
    Color::srgb(0.84, 0.28, 0.46)
}

fn accent_color() -> Color {
    Color::srgb(0.37, 0.75, 0.99)
}

fn accent_hover_color() -> Color {
    Color::srgb(0.5, 0.81, 0.99)
}

fn accent_pressed_color() -> Color {
    Color::srgb(0.3, 0.6, 0.9)
}

fn disabled_accent_color() -> Color {
    Color::srgba(0.5, 0.52, 0.59, 0.7)
}

fn subtle_button_color(alpha: f32) -> Color {
    Color::srgba(0.86, 0.9, 1.0, alpha)
}
