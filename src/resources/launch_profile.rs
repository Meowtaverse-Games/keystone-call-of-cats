use std::path::PathBuf;

use bevy::prelude::*;

use crate::resources::stage_catalog::StageId;

#[derive(Debug, Clone, Default)]
pub enum LaunchType {
    #[default]
    Normal,
    ShowChunkGrammarAsciiMap,
    HeadlessVerify,
    Replay,
    HeadlessMeasureReach,
    SteamAppInfo,
}

#[derive(Resource, Clone, Default)]
pub struct LaunchProfile {
    pub replay_seed: Option<u64>,
    pub replay_speed: Option<f64>,
    pub headless_scenario: Option<PathBuf>,
    pub headless_report: Option<PathBuf>,
    pub headless_jobs: Option<usize>,
    pub headless_viewport_scale: Option<f32>,
    pub changed: bool,
    pub launch_type: LaunchType,
    pub skip_boot: bool,
    pub skip_title: bool,
    pub render_physics: bool,
    pub stage_id: Option<StageId>,
    pub ci_smoke_requested: bool,
    /// An opt-in, machine-readable launch check for CI. This never enables itself
    /// for normal player launches.
    pub ci_smoke_report: Option<PathBuf>,
    pub external_control: bool,
    pub external_control_port: Option<u16>,
    /// Read once from KEYSTONE_EXTERNAL_CONTROL_TOKEN; never printed in the profile.
    pub external_control_token: Option<String>,
}

impl std::fmt::Debug for LaunchProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LaunchProfile")
            .field("changed", &self.changed)
            .field("launch_type", &self.launch_type)
            .field("skip_boot", &self.skip_boot)
            .field("skip_title", &self.skip_title)
            .field("render_physics", &self.render_physics)
            .field("stage_id", &self.stage_id)
            .field("ci_smoke_requested", &self.ci_smoke_requested)
            .field("ci_smoke_report", &self.ci_smoke_report)
            .field("external_control", &self.external_control)
            .field("external_control_port", &self.external_control_port)
            .field(
                "external_control_token",
                &self.external_control_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl LaunchProfile {
    pub fn from_args(args: &[String]) -> Self {
        if args.len() <= 1 {
            return Self::default();
        }
        let mut launch_profile = Self::default();

        let mut changed = false;
        let mut index = 1;

        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "--replay" => {
                    launch_profile.launch_type = LaunchType::Replay;
                    launch_profile.headless_scenario = args.get(index + 1).map(PathBuf::from);
                    index += 1;
                    changed = true;
                }
                "--seed" => {
                    launch_profile.replay_seed = args.get(index + 1).and_then(|s| s.parse().ok());
                    index += 1;
                }
                "--replay-speed" => {
                    launch_profile.replay_speed = Some(
                        args.get(index + 1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(f64::NAN),
                    );
                    index += 1;
                }
                "--headless-verify" => {
                    launch_profile.launch_type = LaunchType::HeadlessVerify;
                    launch_profile.headless_scenario = args.get(index + 1).map(PathBuf::from);
                    index += 1;
                    changed = true;
                }
                "--headless-measure-reach" => {
                    launch_profile.launch_type = LaunchType::HeadlessMeasureReach;
                    changed = true;
                }
                "--report" => {
                    launch_profile.headless_report = args.get(index + 1).map(PathBuf::from);
                    index += 1;
                }
                "--viewport-scale" => {
                    launch_profile.headless_viewport_scale = Some(
                        args.get(index + 1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(f32::NAN),
                    );
                    index += 1;
                }
                "--jobs" => {
                    launch_profile.headless_jobs = Some(
                        args.get(index + 1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0),
                    );
                    index += 1;
                }
                "--show-chunk-grammar-ascii-map" => {
                    launch_profile.launch_type = LaunchType::ShowChunkGrammarAsciiMap;
                    changed = true;
                }
                "--steam-app-info" => {
                    launch_profile.launch_type = LaunchType::SteamAppInfo;
                    changed = true;
                }
                "--skip-boot" => {
                    launch_profile.skip_boot = true;
                    changed = true;
                }
                "--skip-title" => {
                    launch_profile.skip_title = true;
                    changed = true;
                }
                "--render-physics" => {
                    launch_profile.render_physics = true;
                    changed = true;
                }
                "--ci-smoke" => {
                    launch_profile.ci_smoke_requested = true;
                    changed = true;
                }
                "--external-control" => {
                    launch_profile.external_control = true;
                    launch_profile.external_control_token =
                        std::env::var("KEYSTONE_EXTERNAL_CONTROL_TOKEN")
                            .ok()
                            .filter(|v| !v.is_empty());
                    changed = true;
                }
                "--external-control-port" => {
                    if let Some(value) = args.get(index + 1).and_then(|v| v.parse().ok()) {
                        launch_profile.external_control_port = Some(value);
                        index += 1;
                        changed = true;
                    }
                }
                "--ci-smoke-report" => {
                    if let Some(path) = args.get(index + 1) {
                        launch_profile.ci_smoke_report = Some(PathBuf::from(path));
                        changed = true;
                        index += 1;
                    } else {
                        warn!("--ci-smoke-report flag provided without a path");
                    }
                }
                _ if arg.starts_with("--ci-smoke-report=") => {
                    let path = &arg["--ci-smoke-report=".len()..];
                    if path.is_empty() {
                        warn!("--ci-smoke-report flag provided with an empty path");
                    } else {
                        launch_profile.ci_smoke_report = Some(PathBuf::from(path));
                        changed = true;
                    }
                }
                "--debug" => {
                    launch_profile.skip_boot = true;
                    launch_profile.skip_title = true;
                    launch_profile.render_physics = true;
                    changed = true;
                }
                _ if arg.starts_with("--stage-id=") => {
                    let value = &arg["--stage-id=".len()..];
                    match value.parse::<usize>() {
                        Ok(id) => {
                            launch_profile.stage_id = Some(StageId(id));
                            changed = true;
                        }
                        Err(err) => {
                            warn!("Invalid stage id '{value}': {err}");
                        }
                    }
                }
                "--stage-id" => {
                    if index + 1 < args.len() {
                        let value = &args[index + 1];
                        match value.parse::<usize>() {
                            Ok(id) => {
                                launch_profile.stage_id = Some(StageId(id));
                                changed = true;
                                index += 1;
                            }
                            Err(err) => {
                                warn!("Invalid stage id '{value}': {err}");
                                index += 1;
                            }
                        }
                    } else {
                        warn!("--stage-id flag provided without a value");
                    }
                }
                _ => {}
            }
            index += 1;
        }
        launch_profile.changed = changed;

        launch_profile
    }

    pub fn ci_smoke_enabled(&self) -> bool {
        self.ci_smoke_requested && self.ci_smoke_report.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LaunchProfile;

    #[test]
    fn replay_parses_seed_speed_and_optional_report() {
        let profile = LaunchProfile::from_args(
            &[
                "game",
                "--replay",
                "scenario.json",
                "--seed",
                "17",
                "--replay-speed",
                "0.5",
                "--report",
                "report.json",
            ]
            .map(str::to_owned),
        );
        assert!(matches!(profile.launch_type, super::LaunchType::Replay));
        assert_eq!(profile.replay_seed, Some(17));
        assert_eq!(profile.replay_speed, Some(0.5));
        assert_eq!(profile.headless_report, Some(PathBuf::from("report.json")));
        assert!(!profile.external_control);
    }
    #[test]
    fn headless_modes_parse_without_enabling_other_services() {
        let args = [
            "game",
            "--headless-verify",
            "scenario.json",
            "--report",
            "report.json",
            "--jobs",
            "4",
        ]
        .map(str::to_owned);
        let profile = LaunchProfile::from_args(&args);
        assert!(matches!(
            profile.launch_type,
            super::LaunchType::HeadlessVerify
        ));
        assert_eq!(profile.headless_jobs, Some(4));
        assert_eq!(
            profile.headless_scenario,
            Some(PathBuf::from("scenario.json"))
        );
        assert_eq!(profile.headless_report, Some(PathBuf::from("report.json")));
        assert!(!profile.external_control);
        assert!(!profile.ci_smoke_enabled());
        let args =
            ["game", "--headless-measure-reach", "--report", "reach.json"].map(str::to_owned);
        assert!(matches!(
            LaunchProfile::from_args(&args).launch_type,
            super::LaunchType::HeadlessMeasureReach
        ));
        assert!(matches!(
            LaunchProfile::from_args(&["game".into()]).launch_type,
            super::LaunchType::Normal
        ));
    }

    #[test]
    fn parses_ci_smoke_report_path() {
        let args = vec![
            "keystone-cc".to_string(),
            "--ci-smoke".to_string(),
            "--ci-smoke-report".to_string(),
            "result.json".to_string(),
        ];

        let profile = LaunchProfile::from_args(&args);

        assert!(profile.ci_smoke_enabled());
        assert_eq!(
            profile.ci_smoke_report.unwrap(),
            PathBuf::from("result.json")
        );
    }

    #[test]
    fn report_without_smoke_flag_does_not_enable_smoke() {
        let args = vec![
            "keystone-cc".to_string(),
            "--ci-smoke-report=x.json".to_string(),
        ];
        assert!(!LaunchProfile::from_args(&args).ci_smoke_enabled());
    }

    #[test]
    fn smoke_flag_without_report_does_not_enable_smoke() {
        let args = vec!["keystone-cc".to_string(), "--ci-smoke".to_string()];
        assert!(!LaunchProfile::from_args(&args).ci_smoke_enabled());
    }

    #[test]
    fn debug_output_redacts_external_control_token() {
        let profile = LaunchProfile {
            external_control_token: Some("token-that-must-not-be-logged".into()),
            ..LaunchProfile::default()
        };
        let output = format!("{profile:?}");
        assert!(!output.contains("token-that-must-not-be-logged"));
        assert!(output.contains("[REDACTED]"));
    }
}
