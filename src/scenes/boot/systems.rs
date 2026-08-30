use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
#[cfg(target_arch = "wasm32")]
use bevy_fluent::BundleAsset;
use bevy_fluent::prelude::*;

use crate::util::font::apply_font_for_locale;

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

use crate::resources::locale_resources::LocaleAssets;

#[derive(SystemParam)]
pub(crate) struct LocaleLoading<'w> {
    assets: Option<Res<'w, LocaleAssets>>,
    #[cfg(not(target_arch = "wasm32"))]
    builder: LocalizationBuilder<'w>,
    #[cfg(target_arch = "wasm32")]
    locale: Res<'w, Locale>,
    #[cfg(target_arch = "wasm32")]
    bundles: Res<'w, Assets<BundleAsset>>,
}

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

    #[cfg(not(target_arch = "wasm32"))]
    commands.insert_resource(LocaleAssets::Native(asset_server.load_folder("locales")));

    #[cfg(target_arch = "wasm32")]
    commands.insert_resource(LocaleAssets::Web([
        asset_server.load("locales/en-US/main.ftl.ron"),
        asset_server.load("locales/ja-JP/main.ftl.ron"),
        asset_server.load("locales/zh-Hans/main.ftl.ron"),
    ]));

    let mills = if !launch_profile.skip_boot
        && launch_profile.stage_id.is_none()
        && !cfg!(target_arch = "wasm32")
    {
        2400
    } else {
        0
    };
    info!("Boot timer: {}", mills);
    commands.insert_resource(BootTimer {
        // for testing, make it shorter
        timer: Timer::new(Duration::from_millis(mills), TimerMode::Once),
    });
}

pub fn setup_font(
    mut contexts: EguiContexts,
    mut loaded: Local<bool>,
    asset_store: Res<AssetStore>,
    fonts: Res<Assets<Font>>,
    locale: Option<Res<Locale>>,
) {
    if *loaded {
        return;
    }

    let locale_code = if let Some(l) = locale {
        l.requested.to_string()
    } else {
        "en-US".to_string()
    };

    // Attempt to apply font. We will retry next frame if standard font not loaded,
    // unless system font is found.
    // Actually, apply_font_for_locale handles fallback.
    // But we need to ensure fonts are loaded.
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Check if necessary fonts are loaded
    if locale_code == "zh-Hans" && asset_store.font(FontKey::Chinese).is_none() {
        // Wait for Chinese font
        return;
    }
    if asset_store.font(FontKey::Default).is_none() {
        // Wait for Default font (needed for fallback or main)
        return;
    }

    apply_font_for_locale(ctx, &locale_code, &asset_store, &fonts);

    *loaded = true;
}

#[derive(Default)]
pub struct Loaded(bool);

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut reader: MessageReader<AssetGroupLoaded>,
    mut loaded: Local<Loaded>,
    mut localization_failure_reported: Local<bool>,
    mut boot_timer: ResMut<BootTimer>,
    time: Res<Time>,
    scaled_viewport: ResMut<ScaledViewport>,
    mut next_state: ResMut<NextState<GameState>>,
    mut boot_ui: Query<(&BootRoot, &mut Transform)>,
    asset_server: Res<AssetServer>,
    locale_loading: LocaleLoading,
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
        && let Some(locale_assets) = locale_loading.assets.as_ref()
        && let Some(localization_resource) = try_build_localization(
            locale_assets,
            #[cfg(not(target_arch = "wasm32"))]
            &asset_server,
            #[cfg(not(target_arch = "wasm32"))]
            &locale_loading.builder,
            #[cfg(target_arch = "wasm32")]
            &locale_loading.locale,
            #[cfg(target_arch = "wasm32")]
            &locale_loading.bundles,
        )
    {
        commands.insert_resource(localization_resource);
        localization_ready = true;
    } else if !localization_ready
        && let Some(locale_assets) = locale_loading.assets.as_ref()
        && locale_assets.has_failed(&asset_server)
        && !*localization_failure_reported
    {
        error!("Locale assets failed to load; startup will not continue without localization");
        *localization_failure_reported = true;
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

fn try_build_localization(
    locale_assets: &LocaleAssets,
    #[cfg(not(target_arch = "wasm32"))] asset_server: &AssetServer,
    #[cfg(not(target_arch = "wasm32"))] localization_builder: &LocalizationBuilder,
    #[cfg(target_arch = "wasm32")] locale: &Locale,
    #[cfg(target_arch = "wasm32")] bundle_assets: &Assets<BundleAsset>,
) -> Option<Localization> {
    #[cfg(not(target_arch = "wasm32"))]
    return locale_assets.is_loaded(asset_server).then(|| {
        localization_builder.build(
            locale_assets
                .native_folder()
                .expect("native localization assets must be a folder"),
        )
    });

    #[cfg(target_arch = "wasm32")]
    return crate::resources::locale_resources::build_web_localization(
        locale,
        locale_assets
            .web_bundles()
            .expect("web localization assets must be bundles"),
        bundle_assets,
    );
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<BootRoot>>) {
    for ent in query.iter() {
        commands.entity(ent).try_despawn();
    }
}
