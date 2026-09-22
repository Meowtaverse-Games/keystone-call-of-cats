use std::path::PathBuf;

use bevy::prelude::*;

use crate::resources::stage_catalog::StageId;

#[derive(Debug, Clone, Default)]
pub enum LaunchType {
    #[default]
    Normal,
    ShowChunkGrammarAsciiMap,
    SteamAppInfo,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct LaunchProfile {
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
    use super::LaunchProfile;

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
        assert_eq!(profile.ci_smoke_report.unwrap(), "result.json".into());
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
}
