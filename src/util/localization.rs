use bevy_fluent::prelude::Localization;
use fluent_content::Content;

use crate::{resources::stage_catalog::StageMeta, util::script_types::ScriptExecutionError};

pub fn tr(localization: &Localization, key: &str) -> String {
    localization.content(key).unwrap_or_else(|| key.to_string())
}

pub fn tr_or(localization: &Localization, key: &str, fallback: &str) -> String {
    localization
        .content(key)
        .unwrap_or_else(|| fallback.to_string())
}

pub fn tr_with_args(localization: &Localization, key: &str, args: &[(&str, &str)]) -> String {
    if args.is_empty() {
        return tr(localization, key);
    }

    let mut request = String::from(key);
    request.push('?');
    for (index, (name, value)) in args.iter().enumerate() {
        if index > 0 {
            request.push('&');
        }
        request.push_str(name);
        request.push('=');
        request.push_str(value);
    }

    localization
        .content(request.as_str())
        .unwrap_or_else(|| key.to_string())
}

pub fn localized_stage_name(localization: &Localization, stage: &StageMeta) -> String {
    format!(
        "{} {}",
        tr(
            localization,
            &format!("stage-category-{}-name", stage.category)
        ),
        stage.display_number
    )
}

pub fn script_error_message(localization: &Localization, error: &ScriptExecutionError) -> String {
    match error {
        ScriptExecutionError::InvalidMoveDirection { direction } => tr_with_args(
            localization,
            "stage-ui-error-invalid-move-direction",
            &[("direction", direction.as_str())],
        ),
        ScriptExecutionError::InvalidSleepDuration => {
            tr(localization, "stage-ui-error-invalid-sleep-duration")
        }
        ScriptExecutionError::Engine(message) => tr_with_args(
            localization,
            "stage-ui-error-engine",
            &[("message", message.as_str())],
        ),
        ScriptExecutionError::UnsupportedLanguage(language) => tr_with_args(
            localization,
            "stage-ui-error-unsupported-language",
            &[("language", language.as_str())],
        ),
        ScriptExecutionError::InvalidCommand(message) => message.clone(),
    }
}

#[cfg(test)]
mod stage_title_tests {
    use super::*;
    use crate::resources::stage_catalog::StageCatalog;
    use bevy::prelude::*;
    use bevy_fluent::{BundleAsset, FluentPlugin};

    #[test]
    fn category_titles_resolve_in_all_locales_and_use_display_order() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                watch_for_changes_override: Some(false),
                ..default()
            },
            FluentPlugin,
        ));
        let handles: Vec<Handle<BundleAsset>> = ["ja-JP", "en-US", "zh-Hans"]
            .iter()
            .map(|locale| {
                app.world()
                    .resource::<AssetServer>()
                    .load(format!("locales/{locale}/main.ftl.ron"))
            })
            .collect();
        app.finish();
        app.cleanup();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            app.update();
            if handles.iter().all(|handle| {
                app.world()
                    .resource::<Assets<BundleAsset>>()
                    .get(handle)
                    .is_some()
            }) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "locale assets did not load"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let mut stage = StageCatalog::load_from_assets()
            .stage_by_id(&"dig-03".into())
            .unwrap()
            .clone();
        // IDs are permanent; display numbers may differ after filtering/reordering.
        stage.display_number = 42;
        for (handle, expected) in handles.iter().zip(["ほる 42", "Dig 42", "挖掘 42"]) {
            let mut localization = Localization::new();
            localization.insert(
                handle,
                app.world()
                    .resource::<Assets<BundleAsset>>()
                    .get(handle)
                    .unwrap(),
            );
            assert_eq!(localized_stage_name(&localization, &stage), expected);
        }
    }
}
