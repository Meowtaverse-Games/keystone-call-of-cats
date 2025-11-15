use bevy_fluent::prelude::Localization;
use fluent_content::Content;

use crate::{resources::stage_catalog::StageId, util::script_types::ScriptExecutionError};

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

pub fn localized_stage_name(
    localization: &Localization,
    stage_id: StageId,
    fallback: &str,
) -> String {
    let key = format!("stage-name-{}", stage_id.0);
    tr_or(localization, &key, fallback)
}

pub fn script_error_message(localization: &Localization, error: &ScriptExecutionError) -> String {
    match error {
        ScriptExecutionError::EmptyScript => tr(localization, "stage-ui-error-empty-script"),
        ScriptExecutionError::InvalidMoveDirection { direction } => tr_with_args(
            localization,
            "stage-ui-error.invalid-move-direction",
            &[("direction", direction.as_str())],
        ),
        ScriptExecutionError::InvalidSleepDuration => {
            tr(localization, "stage-ui-error-invalid-sleep-duration")
        }
        ScriptExecutionError::Engine(message) => tr_with_args(
            localization,
            "stage-ui-error.engine",
            &[("message", message.as_str())],
        ),
        ScriptExecutionError::UnsupportedLanguage(language) => tr_with_args(
            localization,
            "stage-ui-error.unsupported-language",
            &[("language", language.as_str())],
        ),
        ScriptExecutionError::InvalidCommand(message) => message.clone(),
    }
}
