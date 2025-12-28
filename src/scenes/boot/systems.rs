use std::time::Duration;

use bevy::{
    asset::{LoadState, LoadedFolder},
    prelude::*,
};
use bevy_egui::{EguiContexts, egui};
use bevy_fluent::prelude::*;

use crate::{
    resources::{
        asset_store::{AssetGroupLoaded, AssetStore, LoadAssetGroup},
        design_resolution::ScaledViewport,
        game_state::GameState,
        launch_profile::LaunchProfile,
        stage_catalog::StageCatalog,
    },
    scenes::{
        assets::{DEFAULT_GROUP, FontKey},
        stage::StageProgressionState,
    },
};

use super::components::BootRoot;
#[derive(Resource, Default)]
pub struct BootTimer {
    timer: Timer,
}

#[derive(Resource)]
pub struct LocaleFolder(Handle<LoadedFolder>);

pub fn setup(
    asset_server: Res<AssetServer>,
    scaled_viewport: Res<ScaledViewport>,
    mut commands: Commands,
    mut load_writer: MessageWriter<LoadAssetGroup>,
    launch_profile: Res<LaunchProfile>,
) {
    load_writer.write(DEFAULT_GROUP);

    let fixed_width = 180.0;
    let custom_size = Vec2::new(fixed_width, fixed_width);

    commands.spawn((
        BootRoot,
        Sprite {
            image: asset_server.load("images/logo_with_black.png"),
            custom_size: Some(custom_size),
            ..Default::default()
        },
        Transform::default().with_scale(Vec3::splat(scaled_viewport.scale)),
    ));

    let locale_folder = asset_server.load_folder("locales");
    commands.insert_resource(LocaleFolder(locale_folder));

    let mills = if !launch_profile.skip_boot { 1200 } else { 0 };
    commands.insert_resource(BootTimer {
        // for testing, make it shorter
        timer: Timer::new(Duration::from_millis(mills), TimerMode::Once),
    });
}

const UI_FONT_ID: &str = "pixel_mplus";

pub fn setup_font(
    mut contexts: EguiContexts,
    mut loaded: Local<bool>,
    asset_store: Res<AssetStore>,
    fonts: Res<Assets<Font>>,
) {
    if *loaded {
        return;
    }

    let Some(handle) = asset_store.font(FontKey::Default) else {
        warn_once!("UI font '{}' not found in asset store", UI_FONT_ID);
        return;
    };

    let Some(font) = fonts.get(&handle) else {
        debug!(
            "UI font '{}' handle found, but data not loaded yet",
            UI_FONT_ID
        );
        return;
    };

    info!(
        "UI font '{}' successfully loaded and registering to Egui",
        UI_FONT_ID
    );

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut defs = egui::FontDefinitions::default();
    defs.families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, UI_FONT_ID.to_owned());
    defs.families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, UI_FONT_ID.to_owned());

    defs.font_data.insert(
        UI_FONT_ID.to_owned(),
        egui::FontData::from_owned(font.data.as_ref().clone()).into(),
    );
    ctx.set_fonts(defs);

    *loaded = true;
}

#[derive(Default)]
pub struct Loaded(bool);

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut reader: MessageReader<AssetGroupLoaded>,
    mut loaded: Local<Loaded>,
    mut boot_timer: ResMut<BootTimer>,
    time: Res<Time>,
    scaled_viewport: ResMut<ScaledViewport>,
    mut next_state: ResMut<NextState<GameState>>,
    mut boot_ui: Query<(&BootRoot, &mut Transform)>,
    asset_server: Res<AssetServer>,
    localization_builder: LocalizationBuilder,
    localization_folder: Option<Res<LocaleFolder>>,
    localization: Option<Res<Localization>>,
    launch_profile: Res<LaunchProfile>,
    stage_catalog: Res<StageCatalog>,
    mut progression: ResMut<StageProgressionState>,
) {
    if let Ok((_, mut transform)) = boot_ui.single_mut() {
        transform.scale = Vec3::splat(scaled_viewport.scale);
    }

    for _event in reader.read() {
        info!("Assets loaded event received");
        loaded.0 = true;
    }

    let mut localization_ready = localization.is_some();
    if !localization_ready
        && let Some(folder) = localization_folder
        && matches!(
            asset_server.get_load_state(&folder.0),
            Some(LoadState::Loaded)
        )
    {
        let localization_resource = localization_builder.build(&folder.0);
        commands.insert_resource(localization_resource);
        commands.remove_resource::<LocaleFolder>();
        localization_ready = true;
    }

    boot_timer.timer.tick(time.delta());
    if boot_timer.timer.is_finished() && loaded.0 && localization_ready {
        info!("Boot timer finished");
        let mut target_state = GameState::SelectStage;
        if let Some(stage_id) = launch_profile.stage_id {
            match stage_catalog.stage_by_id(stage_id) {
                Some(stage) => {
                    info!("Launch profile selecting stage {:?}", stage.id);
                    progression.select_stage(stage);
                    target_state = GameState::Stage;
                }
                None => {
                    warn!(
                        "Stage with id {} not found, falling back to select screen",
                        stage_id.0
                    );
                }
            }
        }
        next_state.set(target_state);
    }
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<BootRoot>>) {
    for ent in query.iter() {
        commands.entity(ent).try_despawn();
    }
}
