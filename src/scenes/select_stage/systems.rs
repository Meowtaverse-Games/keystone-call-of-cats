use bevy::{app::AppExit, prelude::*, ui::BorderRadius, window::PrimaryWindow};
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
const ROOT_HORIZONTAL_PADDING: f32 = 48.0;
const CARD_GAP: f32 = 16.0;

#[derive(Resource)]
pub struct StageSelectState {
    current_page: usize,
    cards_per_page: usize,
    total_entries: usize,
    category: usize,
    category_ranges: Vec<std::ops::Range<usize>>,
}

#[derive(Resource, Default)]
pub struct StageTouchInputState {
    active_touch: Option<u64>,
}

type StageTouchButtonComponents<'w> = (
    Entity,
    &'w ComputedNode,
    &'w UiGlobalTransform,
    &'w mut Interaction,
);
type StageTouchButtonFilter = Or<(
    With<StageBackButton>,
    With<StageOptionsButton>,
    With<StagePageButton>,
    With<StageCategoryButton>,
    With<StagePlayButton>,
)>;

impl StageSelectState {
    pub fn new(total_entries: usize, cards_per_page: usize) -> Self {
        Self {
            current_page: 0,
            cards_per_page: cards_per_page.max(1),
            total_entries,
            category: 0,
            category_ranges: std::iter::once(0..total_entries).collect(),
        }
    }

    fn from_catalog(catalog: &StageCatalog, last: Option<&StageId>) -> Self {
        let mut state = Self::new(catalog.stages.len(), CARDS_PER_PAGE);
        let mut offset = 0;
        state.category_ranges = catalog
            .categories
            .iter()
            .filter(|cat| !cat.stages.is_empty())
            .map(|cat| {
                let range = offset..offset + cat.stages.len();
                offset = range.end;
                range
            })
            .collect();
        if state.category_ranges.is_empty() {
            state.category_ranges.push(0..0);
        }
        let index = last
            .and_then(|id| catalog.iter().position(|s| &s.id == id))
            .unwrap_or(0);
        let category = state
            .category_ranges
            .iter()
            .position(|range| range.contains(&index))
            .unwrap_or(0);
        state.select_category(category);
        state.current_page =
            index.saturating_sub(state.category_ranges[category].start) / CARDS_PER_PAGE;
        state
    }
    fn select_category(&mut self, category: usize) {
        if let Some(range) = self.category_ranges.get(category) {
            self.category = category;
            self.total_entries = range.len();
            self.current_page = 0;
        }
    }
    pub fn total_pages(&self) -> usize {
        self.total_entries.div_ceil(self.cards_per_page).max(1)
    }

    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let start = (self.current_page * self.cards_per_page).min(self.total_entries);
        let end = (start + self.cards_per_page).min(self.total_entries);
        let offset = self.category_ranges[self.category].start;
        start + offset..end + offset
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
    fn from(stage_progress: &StageProgress, catalog: &StageCatalog, meta: &StageMeta) -> Self {
        Self {
            meta: meta.clone(),
            playable: stage_progress.is_unlocked(catalog, &meta.id),
        }
    }

    fn localized_title(&self, localization: &Localization) -> String {
        localized_stage_name(localization, &self.meta)
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

    let is_chinese = locale.requested == unic_langid::langid!("zh-Hans");
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
        .map(|m| StageEntry::from(&progress, &catalog, m))
        .collect();
    let summary = StageSummary::from_entries(&entries);
    let state = StageSelectState::from_catalog(&catalog, progress.last_played_stage_id.as_ref());

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
                padding: UiRect::axes(Val::Px(ROOT_HORIZONTAL_PADDING), Val::Px(32.0)),
                // row_gap: Val::Px(SECTION_SPACING * 1.25),
                ..default()
            },
            BackgroundColor(background_color()),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        spawn_glow_layers(parent);
        spawn_hero_section(parent, &font, &display_font, &summary, &localization);
        parent
            .spawn(Node {
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|tabs| {
                for (index, category) in catalog
                    .categories
                    .iter()
                    .filter(|cat| !cat.stages.is_empty())
                    .enumerate()
                {
                    tabs.spawn((
                        Button,
                        StageCategoryButton { index },
                        Node {
                            padding: UiRect::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(card_background_color()),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(tr(
                                &localization,
                                &format!("stage-category-{}-name", category.id),
                            )),
                            TextFont {
                                font: FontSource::Handle(font.clone()),
                                font_size: FontSize::Px(24.0),
                                ..default()
                            },
                            TextColor(primary_text_color()),
                        ));
                    });
                }
            });
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

pub fn handle_category_buttons(
    interactions: Query<(&StageCategoryButton, &Interaction), Changed<Interaction>>,
    mut state: ResMut<StageSelectState>,
    options: Res<OptionsOverlayState>,
) {
    if options.open {
        return;
    }
    for (button, interaction) in &interactions {
        if *interaction == Interaction::Pressed {
            state.select_category(button.index);
        }
    }
}

pub fn update_category_visuals(
    state: Res<StageSelectState>,
    mut buttons: Query<(&StageCategoryButton, &Interaction, &mut BackgroundColor)>,
) {
    for (button, interaction, mut color) in &mut buttons {
        color.0 = if button.index == state.category {
            accent_color()
        } else if *interaction == Interaction::Hovered {
            subtle_button_color(0.35)
        } else {
            card_background_color()
        };
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
                stage_progress.set_last_played(stage.id.clone(), &**file_storage);
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
    mut buttons: Query<(&StagePlayButton, &mut Interaction)>,
) {
    if options.open {
        return;
    }
    let slot = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .iter()
        .position(|key| keys.just_pressed(*key))
        .or_else(|| keys.just_pressed(KeyCode::Enter).then_some(0));
    if let Some(slot) = slot {
        let index = state.visible_range().start + slot;
        for (button, mut interaction) in &mut buttons {
            if button.enabled
                && button.stage_index == index
                && state.visible_range().contains(&index)
            {
                *interaction = Interaction::Pressed;
            }
        }
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        let category = (state.category + 1).min(state.category_ranges.len() - 1);
        state.select_category(category);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        let category = state.category.saturating_sub(1);
        state.select_category(category);
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

/// Routes touch starts using their own coordinates instead of a potentially stale mouse cursor.
///
/// Bevy's standard UI focus system falls back to touch coordinates only when no cursor position
/// is available. On a touchscreen PC, a cursor can remain at an unrelated location after mouse
/// use. Normalizing the interaction after focus has run keeps the existing button handlers and
/// mouse behavior intact while ensuring one stage-select action receives the touch.
pub fn route_touch_to_stage_button(
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    options: Res<OptionsOverlayState>,
    mut touch_state: ResMut<StageTouchInputState>,
    mut buttons: Query<StageTouchButtonComponents<'_>, StageTouchButtonFilter>,
) {
    if options.open {
        clear_stage_button_interactions(&mut buttons);
        touch_state.active_touch = None;
        return;
    }

    if let Some(active_touch) = touch_state.active_touch {
        clear_stage_button_interactions(&mut buttons);
        let touch_finished = touches.just_released(active_touch)
            || touches.just_canceled(active_touch)
            || touches.get_pressed(active_touch).is_none();
        if touch_finished {
            touch_state.active_touch = None;
        } else {
            return;
        }
    }

    if !touches.any_just_pressed() {
        if touches.any_just_canceled() {
            clear_stage_button_interactions(&mut buttons);
        }
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(touch) = touches.iter_just_pressed().min_by_key(|touch| touch.id()) else {
        return;
    };

    // Touch positions are logical pixels; computed UI positions are physical pixels.
    let touch_position = touch.position() * window.scale_factor();
    let mut target = None;

    for (entity, node, transform, mut interaction) in &mut buttons {
        if target.is_none() && node.contains_point(*transform, touch_position) {
            target = Some(entity);
        }
        if *interaction != Interaction::None {
            *interaction = Interaction::None;
        }
    }

    if let Some(entity) = target
        && let Ok((_, _, _, mut interaction)) = buttons.get_mut(entity)
    {
        *interaction = Interaction::Pressed;
        touch_state.active_touch = Some(touch.id());
    }
}

fn clear_stage_button_interactions(
    buttons: &mut Query<StageTouchButtonComponents<'_>, StageTouchButtonFilter>,
) {
    for (_, _, _, mut interaction) in buttons.iter_mut() {
        if *interaction != Interaction::None {
            *interaction = Interaction::None;
        }
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
            border_radius: BorderRadius::all(Val::Px(360.0)),
            ..default()
        },
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
            border_radius: BorderRadius::all(Val::Px(420.0)),
            ..default()
        },
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
            // Keep the header content-sized so the card grid and page controls retain space.
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
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(badge_background_color()),
        ))
        .with_children(|badge| {
            badge
                .spawn(Text::new(label))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(22.0),
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
                        font: FontSource::Handle(display_font.clone()),
                        font_size: FontSize::Px(56.0),
                        ..default()
                    },
                    TextColor(hero_shadow_color()),
                ));

                stack.spawn((
                    Text::new(inline_title),
                    TextFont {
                        font: FontSource::Handle(display_font.clone()),
                        font_size: FontSize::Px(56.0),
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
                border_radius: BorderRadius::all(Val::Px(18.0)),
                ..default()
            },
            BackgroundColor(stat_card_background(accent)),
        ))
        .with_children(|card| {
            card.spawn(Text::new(label))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(16.0),
                    ..default()
                })
                .insert(TextColor(secondary_text_color()));

            card.spawn(Text::new(value))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(32.0),
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
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(initial),
        ))
        .with_children(|btn| {
            let label = tr(localization, "stage-select-back");
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(24.0),
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
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(initial),
        ))
        .with_children(|btn| {
            let label = tr(localization, "stage-select-options");
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(24.0),
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
            flex_wrap: FlexWrap::NoWrap,
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
                        // Equal flex shares keep all cards from the current page in one row.
                        width: Val::Px(0.0),
                        min_width: Val::Px(0.0),
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
                        border_radius: BorderRadius::all(Val::Px(28.0)),
                        ..default()
                    },
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
                        let stage_number = format!("{:02}", entry.meta.display_number);
                        let stage_header = tr_with_args(
                            localization,
                            "stage-select-stage-header",
                            &[("number", &stage_number)],
                        );
                        header
                            .spawn(Text::new(stage_header))
                            .insert(TextFont {
                                font: FontSource::Handle(font.clone()),
                                font_size: FontSize::Px(18.0),
                                ..default()
                            })
                            .insert(TextColor(secondary_text_color()));

                        spawn_stage_chip(header, font, entry.playable, &ready_label, &locked_label);
                    });

                    if !entry.meta.published {
                        spawn_experimental_label(
                            card,
                            font,
                            &tr(localization, "stage-select-badge-experimental"),
                            true,
                        );
                    }
                    let stage_title = entry.localized_title(localization);
                    card.spawn(Text::new(stage_title))
                        .insert(TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(32.0),
                            ..default()
                        })
                        .insert(TextColor(primary_text_color()));

                    card.spawn((
                        Node {
                            flex_grow: 1.0,
                            border_radius: BorderRadius::all(Val::Px(20.0)),
                            ..default()
                        },
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
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(color),
        ))
        .with_children(|chip| {
            chip.spawn(Text::new(label.to_string()))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(14.0),
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
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(button_initial_color(&visual)),
        ))
        .with_children(|btn| {
            btn.spawn(Text::new(label.to_string()))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(24.0),
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
                border_radius: BorderRadius::all(Val::Px(28.0)),
                ..default()
            },
            BackgroundColor(nav_background_color()),
            BorderColor::all(card_border_color()),
        ))
        .with_children(|nav| {
            spawn_nav_button(nav, font, "<", -1);
            nav.spawn((
                StagePageIndicator,
                Text::new(initial_value),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(28.0),
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
                border_radius: BorderRadius::all(Val::Px(18.0)),
                ..default()
            },
            BackgroundColor(button_initial_color(&visual)),
        ))
        .with_children(|btn| {
            btn.spawn(Text::new(label))
                .insert(TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(24.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        input::{
            InputPlugin,
            touch::{TouchInput, TouchPhase},
        },
        window::WindowResolution,
    };

    const MINIMUM_DESKTOP_VIEWPORT_WIDTH: f32 = 960.0;

    #[test]
    fn minimum_desktop_viewport_assigns_each_card_a_positive_flex_share() {
        let available_width = MINIMUM_DESKTOP_VIEWPORT_WIDTH - ROOT_HORIZONTAL_PADDING * 2.0;
        let card_width = (available_width - CARD_GAP * (CARDS_PER_PAGE.saturating_sub(1)) as f32)
            / CARDS_PER_PAGE as f32;

        assert!(
            card_width >= 260.0,
            "each stage card must retain a usable flex share within the minimum desktop viewport"
        );
    }

    #[test]
    fn restores_category_page_and_hides_empty_categories() {
        let catalog = StageCatalog::load_from_assets();
        let mut state = StageSelectState::from_catalog(&catalog, Some(&"dig-04".into()));
        assert_eq!(state.category_ranges.len(), 3);
        assert_eq!(state.category, 2);
        assert_eq!(state.current_page, 1);
        assert_eq!(state.visible_range(), 11..12);
        state.select_category(0);
        assert_eq!(state.visible_range(), 0..3);
        assert!(state.move_page(1));
        assert_eq!(state.visible_range(), 3..6);
        assert!(!state.move_page(1));
        state.select_category(1);
        assert_eq!(state.visible_range(), 6..8);
        assert_eq!(state.total_pages(), 1);
        let progress = StageProgress::default();
        assert!(
            catalog.categories[1]
                .stages
                .iter()
                .all(|s| !progress.is_unlocked(&catalog, &s.id))
        );
    }

    #[test]
    fn mouse_and_keyboard_select_categories_and_keyboard_only_plays_visible_unlocked_cards() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(StageSelectState::from_catalog(
            &StageCatalog::load_from_assets(),
            None,
        ));
        world.insert_resource(OptionsOverlayState::default());
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.init_resource::<Messages<AppExit>>();
        let tab = world
            .spawn((StageCategoryButton { index: 2 }, Interaction::Pressed))
            .id();
        world.run_system_once(handle_category_buttons).unwrap();
        assert_eq!(world.resource::<StageSelectState>().category, 2);
        *world.get_mut::<Interaction>(tab).unwrap() = Interaction::None;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        world.run_system_once(handle_keyboard_navigation).unwrap();
        assert_eq!(world.resource::<StageSelectState>().category, 1);
        let locked = world
            .spawn((
                StagePlayButton {
                    stage_index: 6,
                    enabled: false,
                },
                Interaction::None,
            ))
            .id();
        world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        world.run_system_once(handle_keyboard_navigation).unwrap();
        assert_eq!(
            *world.get::<Interaction>(locked).unwrap(),
            Interaction::None
        );
        world.resource_mut::<StageSelectState>().select_category(0);
        let playable = world
            .spawn((
                StagePlayButton {
                    stage_index: 0,
                    enabled: true,
                },
                Interaction::None,
            ))
            .id();
        world.run_system_once(handle_keyboard_navigation).unwrap();
        assert_eq!(
            *world.get::<Interaction>(playable).unwrap(),
            Interaction::Pressed
        );
    }

    #[test]
    fn mouse_keyboard_and_touch_play_the_same_category_card() {
        use crate::resources::file_storage::{FileError, FileStorage};
        struct NoSave;
        impl FileStorage for NoSave {
            fn load(&self, _: &str) -> Result<Option<Vec<u8>>, FileError> {
                Ok(None)
            }
            fn save(&self, _: &str, _: &[u8]) -> Result<(), FileError> {
                Ok(())
            }
        }
        for mode in ["mouse", "keyboard", "touch"] {
            let catalog = StageCatalog::load_from_assets();
            let state = StageSelectState::from_catalog(&catalog, Some(&"dig-02".into()));
            let mut app = App::new();
            app.add_plugins(InputPlugin)
                .insert_resource(catalog)
                .insert_resource(state)
                .init_resource::<OptionsOverlayState>()
                .init_resource::<StageTouchInputState>()
                .init_resource::<StageProgressionState>()
                .init_resource::<StageProgress>()
                .init_resource::<NextState<GameState>>()
                .init_resource::<AudioHandles>()
                .init_resource::<GameSettings>()
                .insert_resource(FileStorageResource::new(std::sync::Arc::new(NoSave)))
                .add_message::<AppExit>()
                .add_systems(
                    Update,
                    (
                        route_touch_to_stage_button,
                        handle_keyboard_navigation,
                        handle_play_buttons,
                    )
                        .chain(),
                );
            let window = app
                .world_mut()
                .spawn((PrimaryWindow, Window::default()))
                .id();
            let target = app
                .world_mut()
                .spawn((
                    StagePlayButton {
                        stage_index: 9,
                        enabled: true,
                    },
                    button_node(Vec2::new(100.0, 100.0)),
                ))
                .id();
            match mode {
                "mouse" => {
                    *app.world_mut().get_mut::<Interaction>(target).unwrap() = Interaction::Pressed
                }
                "keyboard" => {
                    app.world_mut()
                        .write_message(bevy::input::keyboard::KeyboardInput {
                            key_code: KeyCode::Digit2,
                            logical_key: bevy::input::keyboard::Key::Character("2".into()),
                            state: bevy::input::ButtonState::Pressed,
                            text: None,
                            repeat: false,
                            window,
                        });
                }
                _ => {
                    app.world_mut().write_message(touch_input(
                        window,
                        1,
                        TouchPhase::Started,
                        Vec2::new(100.0, 100.0),
                    ));
                }
            }
            app.update();
            assert_eq!(
                app.world()
                    .resource::<StageProgressionState>()
                    .current_stage_id(),
                StageId::from("dig-02"),
                "{mode}"
            );
            assert_eq!(
                app.world().resource::<StageProgress>().last_played_stage_id,
                Some("dig-02".into()),
                "{mode}"
            );
            assert!(matches!(
                app.world().resource::<NextState<GameState>>(),
                NextState::Pending(GameState::Stage)
            ));
        }
    }

    fn touch_input(window: Entity, id: u64, phase: TouchPhase, position: Vec2) -> TouchInput {
        TouchInput {
            phase,
            position,
            window,
            force: None,
            id,
        }
    }

    fn button_node(center: Vec2) -> (ComputedNode, UiGlobalTransform, Interaction) {
        let mut node = ComputedNode::default();
        node.size = Vec2::splat(100.0);
        (
            node,
            UiGlobalTransform::from_translation(center),
            Interaction::None,
        )
    }

    #[test]
    fn touch_uses_its_scaled_position_instead_of_a_stale_button_interaction() {
        let mut app = App::new();
        app.add_plugins(InputPlugin)
            .init_resource::<OptionsOverlayState>()
            .init_resource::<StageTouchInputState>()
            .add_systems(Update, route_touch_to_stage_button);

        let window = app
            .world_mut()
            .spawn((
                PrimaryWindow,
                Window {
                    resolution: WindowResolution::new(1920, 1200).with_scale_factor_override(1.75),
                    ..default()
                },
            ))
            .id();
        let target = app
            .world_mut()
            .spawn((
                StageCategoryButton { index: 1 },
                button_node(Vec2::new(350.0, 175.0)),
            ))
            .id();
        let stale = app
            .world_mut()
            .spawn((
                StagePageButton { delta: -1 },
                button_node(Vec2::new(50.0, 50.0)),
            ))
            .id();
        *app.world_mut().get_mut::<Interaction>(stale).unwrap() = Interaction::Pressed;

        app.world_mut().write_message(touch_input(
            window,
            1,
            TouchPhase::Started,
            Vec2::new(200.0, 100.0),
        ));
        app.update();

        assert_eq!(
            *app.world().get::<Interaction>(target).unwrap(),
            Interaction::Pressed
        );
        assert_eq!(
            *app.world().get::<Interaction>(stale).unwrap(),
            Interaction::None
        );
    }

    #[test]
    fn released_or_canceled_touch_clears_the_captured_button() {
        let mut app = App::new();
        app.add_plugins(InputPlugin)
            .init_resource::<OptionsOverlayState>()
            .init_resource::<StageTouchInputState>()
            .add_systems(Update, route_touch_to_stage_button);

        let window = app
            .world_mut()
            .spawn((PrimaryWindow, Window::default()))
            .id();
        let button = app
            .world_mut()
            .spawn((StagePageButton { delta: 1 }, button_node(Vec2::ZERO)))
            .id();

        for phase in [
            TouchPhase::Started,
            TouchPhase::Ended,
            TouchPhase::Started,
            TouchPhase::Canceled,
        ] {
            app.world_mut()
                .write_message(touch_input(window, 1, phase, Vec2::ZERO));
            app.update();
            if matches!(phase, TouchPhase::Started) {
                assert_eq!(
                    *app.world().get::<Interaction>(button).unwrap(),
                    Interaction::Pressed
                );
                continue;
            }
            assert_eq!(
                *app.world().get::<Interaction>(button).unwrap(),
                Interaction::None
            );
        }
    }

    #[test]
    fn missing_captured_touch_allows_a_new_tap_in_the_same_frame() {
        let mut app = App::new();
        app.add_plugins(InputPlugin)
            .init_resource::<OptionsOverlayState>()
            .init_resource::<StageTouchInputState>()
            .add_systems(Update, route_touch_to_stage_button);

        let window = app
            .world_mut()
            .spawn((PrimaryWindow, Window::default()))
            .id();
        let button = app
            .world_mut()
            .spawn((StagePageButton { delta: 1 }, button_node(Vec2::ZERO)))
            .id();
        app.world_mut()
            .resource_mut::<StageTouchInputState>()
            .active_touch = Some(99);

        // The touch ended while another game state was active, so it is absent here.
        app.world_mut()
            .write_message(touch_input(window, 1, TouchPhase::Started, Vec2::ZERO));
        app.update();
        assert_eq!(
            *app.world().get::<Interaction>(button).unwrap(),
            Interaction::Pressed
        );
    }

    #[test]
    fn released_touch_allows_the_next_tap_in_the_same_frame() {
        let mut app = App::new();
        app.add_plugins(InputPlugin)
            .init_resource::<OptionsOverlayState>()
            .init_resource::<StageTouchInputState>()
            .add_systems(Update, route_touch_to_stage_button);

        let window = app
            .world_mut()
            .spawn((PrimaryWindow, Window::default()))
            .id();
        let first = app
            .world_mut()
            .spawn((
                StagePageButton { delta: -1 },
                button_node(Vec2::new(-100.0, 0.0)),
            ))
            .id();
        let second = app
            .world_mut()
            .spawn((
                StagePageButton { delta: 1 },
                button_node(Vec2::new(100.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(touch_input(
            window,
            1,
            TouchPhase::Started,
            Vec2::new(-100.0, 0.0),
        ));
        app.update();
        assert_eq!(
            *app.world().get::<Interaction>(first).unwrap(),
            Interaction::Pressed
        );

        app.world_mut().write_message(touch_input(
            window,
            1,
            TouchPhase::Ended,
            Vec2::new(-100.0, 0.0),
        ));
        app.world_mut().write_message(touch_input(
            window,
            2,
            TouchPhase::Started,
            Vec2::new(100.0, 0.0),
        ));
        app.update();

        assert_eq!(
            *app.world().get::<Interaction>(first).unwrap(),
            Interaction::None
        );
        assert_eq!(
            *app.world().get::<Interaction>(second).unwrap(),
            Interaction::Pressed
        );
    }

    #[test]
    fn first_touch_is_captured_and_later_touches_do_not_press_another_button() {
        let mut app = App::new();
        app.add_plugins(InputPlugin)
            .init_resource::<OptionsOverlayState>()
            .init_resource::<StageTouchInputState>()
            .add_systems(Update, route_touch_to_stage_button);

        let window = app
            .world_mut()
            .spawn((PrimaryWindow, Window::default()))
            .id();
        let first = app
            .world_mut()
            .spawn((
                StagePageButton { delta: -1 },
                button_node(Vec2::new(100.0, 0.0)),
            ))
            .id();
        let second = app
            .world_mut()
            .spawn((
                StagePageButton { delta: 1 },
                button_node(Vec2::new(-100.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(touch_input(
            window,
            1,
            TouchPhase::Started,
            Vec2::new(100.0, 0.0),
        ));
        app.world_mut().write_message(touch_input(
            window,
            2,
            TouchPhase::Started,
            Vec2::new(-100.0, 0.0),
        ));
        app.update();

        assert_eq!(
            *app.world().get::<Interaction>(first).unwrap(),
            Interaction::Pressed
        );
        assert_eq!(
            *app.world().get::<Interaction>(second).unwrap(),
            Interaction::None
        );
    }

    #[test]
    fn options_overlay_blocks_stage_button_touches() {
        let mut app = App::new();
        app.add_plugins(InputPlugin)
            .insert_resource(OptionsOverlayState {
                open: true,
                ..default()
            })
            .init_resource::<StageTouchInputState>()
            .add_systems(Update, route_touch_to_stage_button);

        let window = app
            .world_mut()
            .spawn((PrimaryWindow, Window::default()))
            .id();
        let button = app
            .world_mut()
            .spawn((StagePageButton { delta: 1 }, button_node(Vec2::ZERO)))
            .id();

        app.world_mut()
            .write_message(touch_input(window, 1, TouchPhase::Started, Vec2::ZERO));
        app.update();

        assert_eq!(
            *app.world().get::<Interaction>(button).unwrap(),
            Interaction::None
        );
    }
}
fn subtle_button_color(alpha: f32) -> Color {
    Color::srgba(0.86, 0.9, 1.0, alpha)
}
