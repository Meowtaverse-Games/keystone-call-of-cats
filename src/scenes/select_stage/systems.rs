use bevy::{app::AppExit, prelude::*, ui::BorderRadius};
use bevy_ecs::hierarchy::ChildSpawnerCommands;
use bevy_fluent::prelude::{Locale, Localization};

use super::components::*;
use crate::{
    resources::{
        asset_store::AssetStore,
        design_resolution::{LetterboxOffsets, LetterboxVisibility},
        file_storage::FileStorageResource,
        game_state::GameState,
        settings::GameSettings,
        stage_catalog::*,
        stage_progress::*,
    },
    scenes::{
        assets::FontKey,
        audio::{AudioHandles, play_bgm, play_ui_click},
        options::OptionsOverlayState,
        stage::StageProgressionState,
    },
    util::localization::{localized_stage_name, tr, tr_with_args},
};

const CARDS_PER_PAGE: usize = 3;
const CARD_WIDTH: f32 = 360.0;
const CARD_GAP: f32 = 32.0;

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
    meta: StageMeta,
    playable: bool,
}

impl StageEntry {
    fn from(stage_progress: &StageProgress, meta: &StageMeta) -> Self {
        Self {
            meta: meta.clone(),
            playable: stage_progress.is_unlocked(meta.id),
        }
    }

    fn localized_title(&self, localization: &Localization) -> String {
        localized_stage_name(localization, self.meta.id, &self.meta.title)
    }
}

struct StageSummary {
    total: usize,
    unlocked: usize,
    locked: usize,
}

impl StageSummary {
    fn from_entries(entries: &[StageEntry]) -> Self {
        let total = entries.len();
        let unlocked = entries.iter().filter(|entry| entry.playable).count();
        let locked = total.saturating_sub(unlocked);

        Self {
            total,
            unlocked,
            locked,
        }
    }
}

pub fn setup_bgm(
    mut commands: Commands,
    mut audio: ResMut<AudioHandles>,
    settings: Res<GameSettings>,
) {
    play_bgm(&mut commands, &mut audio, &settings);
}

#[allow(clippy::too_many_arguments)]
pub fn setup(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    mut letterbox_offsets: ResMut<LetterboxOffsets>,
    mut letterbox_visibility: ResMut<LetterboxVisibility>,
    asset_store: Res<AssetStore>,
    catalog: Res<StageCatalog>,
    progress: Res<StageProgress>,
    localization: Res<Localization>,
    mut options_overlay: ResMut<OptionsOverlayState>,
    locale: Res<Locale>,
) {
    clear_color.0 = background_color();
    letterbox_offsets.left = 0.0;
    letterbox_offsets.right = 0.0;
    letterbox_visibility.0 = false; // Hide letterbox only in SelectStage
    options_overlay.open = false;

    let is_chinese = locale.requested.to_string() == "zh-Hans";
    let font_key = if is_chinese {
        FontKey::Chinese
    } else {
        FontKey::Default
    };

    let font = if let Some(handle) = asset_store.font(font_key) {
        handle
    } else {
        warn!(
            "StageSelect: font {:?} is not available, UI text may be missing",
            font_key
        );
        Handle::default()
    };

    let display_font = if is_chinese {
        // Use the same Chinese font for titles as Quicky Story likely lacks CJK
        asset_store
            .font(FontKey::Chinese)
            .unwrap_or_else(|| font.clone())
    } else {
        asset_store
            .font(FontKey::Title)
            .unwrap_or_else(|| font.clone())
    };

    let entries: Vec<StageEntry> = catalog
        .iter()
        .map(|m| StageEntry::from(&progress, m))
        .collect();
    let summary = StageSummary::from_entries(&entries);
    let mut state = StageSelectState::new(entries.len(), CARDS_PER_PAGE);

    // Restore last played page
    if let Some(index) = progress
        .last_played_stage_id
        .and_then(|last_id| catalog.iter().position(|m| m.id == last_id))
    {
        state.current_page = index / CARDS_PER_PAGE;
    }

    let current_page_number = state.current_page + 1;
    let total_pages = state.total_pages();
    let page_text = format!("{}/{}", current_page_number, total_pages);
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
                // row_gap: Val::Px(SECTION_SPACING * 1.25),
                ..default()
            },
            BackgroundColor(background_color()),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        spawn_glow_layers(parent);
        spawn_hero_section(parent, &font, &display_font, &summary, &localization);
        spawn_stage_cards(parent, &entries, &font, &localization);
        spawn_bottom_bar(parent, &font, &page_text);
    });
}

pub fn cleanup(
    mut commands: Commands,
    roots: Query<Entity, With<StageSelectRoot>>,
    // bgm_entities: Query<Entity, With<StageSelectBgm>>,
    mut letterbox_visibility: ResMut<LetterboxVisibility>,
) {
    for entity in roots.iter() {
        commands.entity(entity).try_despawn();
    }

    // for entity in bgm_entities.iter() {
    //     commands.entity(entity).despawn();
    // }

    commands.remove_resource::<StageSelectState>();
    // Restore visibility for other scenes
    letterbox_visibility.0 = true;
}

pub fn handle_back_button(
    mut commands: Commands,
    audio: Res<AudioHandles>,
    settings: Res<GameSettings>,
    mut interactions: Query<(&StageBackButton, &Interaction), Changed<Interaction>>,
    mut exit_events: MessageWriter<AppExit>,
    options: Res<OptionsOverlayState>,
) {
    if options.open {
        return;
    }
    for (_, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            play_ui_click(&mut commands, &audio, &settings);
            exit_events.write(AppExit::Success);
        }
    }
}

pub fn handle_nav_buttons(
    mut commands: Commands,
    audio: Res<AudioHandles>,
    settings: Res<GameSettings>,
    mut interactions: Query<(&StagePageButton, &Interaction), Changed<Interaction>>,
    mut state: ResMut<StageSelectState>,
    options: Res<OptionsOverlayState>,
) {
    if options.open {
        return;
    }
    for (button, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            play_ui_click(&mut commands, &audio, &settings);
            state.move_page(button.delta);
        }
    }
}

pub fn handle_options_button(
    mut commands: Commands,
    audio: Res<AudioHandles>,
    settings: Res<GameSettings>,
    mut interactions: Query<(&StageOptionsButton, &Interaction), Changed<Interaction>>,
    mut overlay: ResMut<OptionsOverlayState>,
    time: Res<Time>,
) {
    for (_, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            play_ui_click(&mut commands, &audio, &settings);
            overlay.open = true;
            overlay.opened_at = time.elapsed_secs_f64();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_play_buttons(
    mut commands: Commands,
    audio: Res<AudioHandles>,
    settings: Res<GameSettings>,
    mut interactions: Query<(&StagePlayButton, &Interaction), Changed<Interaction>>,
    mut progression: ResMut<StageProgressionState>,
    catalog: Res<StageCatalog>,
    mut next_state: ResMut<NextState<GameState>>,
    options: Res<OptionsOverlayState>,
    mut stage_progress: ResMut<StageProgress>,
    file_storage: Res<FileStorageResource>,
) {
    if options.open {
        return;
    }
    for (button, interaction) in &mut interactions {
        if !button.enabled {
            continue;
        }

        if *interaction == Interaction::Pressed {
            play_ui_click(&mut commands, &audio, &settings);
            if let Some(stage) = catalog.stage_by_index(button.stage_index) {
                stage_progress.set_last_played(stage.id, &**file_storage);
                progression.select_stage(stage);
                next_state.set(GameState::Stage);
            } else {
                warn!("Stage {} is not available", button.stage_index + 1);
            }
        }
    }
}

pub fn handle_keyboard_navigation(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<StageSelectState>,
    mut exit_events: MessageWriter<AppExit>,
    options: Res<OptionsOverlayState>,
) {
    if options.open {
        return;
    }
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
    localization: &Localization,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
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
                if cfg!(feature = "experimental") {
                    let label = tr(localization, "stage-select-badge-experimental");
                    spawn_experimental_label(row, font, &label, true);
                } else {
                    spawn_experimental_label(row, font, "", false);
                }

                row.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|buttons| {
                    spawn_options_button(buttons, font, localization);
                    spawn_back_button(buttons, font, localization);
                });
            });

            hero.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(32.0),
                align_items: AlignItems::Stretch,
                ..default()
            })
            .with_children(|content| {
                spawn_hero_copy(content, font, display_font, summary, localization);
            });
        });
}

fn spawn_experimental_label(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    is_show: bool,
) {
    if !is_show {
        parent.spawn((Node {
            padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
            ..default()
        },));
        return;
    }
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
    localization: &Localization,
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
                let inline_title = tr(localization, "game-title-inline");
                stack.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(4.0),
                        ..default()
                    },
                    Text::new(inline_title.clone()),
                    TextFont {
                        font: display_font.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(hero_shadow_color()),
                ));

                stack.spawn((
                    Text::new(inline_title),
                    TextFont {
                        font: display_font.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(hero_title_color()),
                ));
            });

            left.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|stats| {
                let unlocked_label = tr(localization, "stage-select-stats-unlocked");
                let locked_label = tr(localization, "stage-select-stats-locked");
                let slots_label = tr(localization, "stage-select-stats-slots");
                spawn_stat_card(
                    stats,
                    font,
                    &unlocked_label,
                    &format!("{}", summary.unlocked),
                    true,
                );
                spawn_stat_card(
                    stats,
                    font,
                    &locked_label,
                    &format!("{}", summary.locked),
                    false,
                );
                spawn_stat_card(
                    stats,
                    font,
                    &slots_label,
                    &format!("{}", summary.total),
                    false,
                );
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

fn spawn_back_button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    localization: &Localization,
) {
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
            let label = tr(localization, "stage-select-back");
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_options_button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    localization: &Localization,
) {
    let visual = ButtonVisual::new(
        subtle_button_color(0.25),
        subtle_button_color(0.4),
        subtle_button_color(0.55),
        subtle_button_color(0.1),
        true,
    );
    let initial = button_initial_color(&visual);

    parent
        .spawn((
            StageOptionsButton,
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
            let label = tr(localization, "stage-select-options");
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_stage_cards(
    parent: &mut ChildSpawnerCommands,
    entries: &[StageEntry],
    font: &Handle<Font>,
    localization: &Localization,
) {
    let ready_label = tr(localization, "stage-select-state-ready");
    let locked_label = tr(localization, "stage-select-state-locked");
    let play_label = tr(localization, "stage-select-play");
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(CARD_GAP),
            column_gap: Val::Px(CARD_GAP),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            margin: UiRect {
                top: Val::Px(20.0),
                ..default()
            },
            ..default()
        })
        .with_children(|grid| {
            for (index, entry) in entries.iter().enumerate() {
                grid.spawn((
                    StageCard { index },
                    Node {
                        width: Val::Px(CARD_WIDTH),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        margin: UiRect {
                            bottom: Val::Px(12.0),
                            ..default()
                        },
                        border: UiRect::all(Val::Px(2.0)),
                        display: if index < CARDS_PER_PAGE {
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
                        let stage_number = format!("{:02}", index + 1);
                        let stage_header = tr_with_args(
                            localization,
                            "stage-select-stage-header",
                            &[("number", stage_number.as_str())],
                        );
                        header
                            .spawn(Text::new(stage_header))
                            .insert(TextFont {
                                font: font.clone(),
                                font_size: 18.0,
                                ..default()
                            })
                            .insert(TextColor(secondary_text_color()));

                        spawn_stage_chip(header, font, entry.playable, &ready_label, &locked_label);
                    });

                    let stage_title = entry.localized_title(localization);
                    card.spawn(Text::new(stage_title))
                        .insert(TextFont {
                            font: font.clone(),
                            font_size: 32.0,
                            ..default()
                        })
                        .insert(TextColor(primary_text_color()));

                    card.spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        BorderRadius::all(Val::Px(20.0)),
                        BackgroundColor(preview_background()),
                    ))
                    .with_children(|_| {});

                    spawn_play_button(card, index, entry, font, &play_label, &locked_label);
                });
            }
        });
}

fn spawn_stage_chip(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    playable: bool,
    ready_label: &str,
    locked_label: &str,
) {
    let (label, color) = if playable {
        (ready_label, hero_button_color())
    } else {
        (locked_label, disabled_accent_color())
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
            chip.spawn(Text::new(label.to_string()))
                .insert(TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                })
                .insert(TextColor(primary_text_color()));
        });
}

fn spawn_play_button(
    parent: &mut ChildSpawnerCommands,
    stage_index: usize,
    entry: &StageEntry,
    font: &Handle<Font>,
    play_label: &str,
    locked_label: &str,
) {
    let enabled = entry.playable;
    let visual = ButtonVisual::new(
        accent_color(),
        accent_hover_color(),
        accent_pressed_color(),
        disabled_accent_color(),
        enabled,
    );

    let label = if enabled { play_label } else { locked_label };

    parent
        .spawn((
            StagePlayButton {
                stage_index,
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
            btn.spawn(Text::new(label.to_string()))
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
                padding: UiRect::axes(Val::Px(8.0), Val::Px(8.0)),
                margin: UiRect {
                    bottom: Val::Px(24.0),
                    ..default()
                },
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
