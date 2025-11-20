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
}
