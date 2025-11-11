use bevy::prelude::*;

#[derive(Debug, Clone, Default)]
pub enum LaunchType {
    #[default]
    Normal,
    GenerateChunkGrammerMap,
    SteamAppInfo,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct LaunchProfile {
    pub changed: bool,
    pub launch_type: LaunchType,
    pub skip_boot: bool,
    pub skip_title: bool,
    pub render_physics: bool,
}

impl LaunchProfile {
    pub fn from_args(args: &[String]) -> Self {
        if args.len() <= 1 {
            return Self::default();
        }
        let mut launch_profile = Self::default();

        let mut not_changed_count = 0;

        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--chunk-grammar-map" => {
                    launch_profile.launch_type = LaunchType::GenerateChunkGrammerMap
                }
                "--steam-app-info" => launch_profile.launch_type = LaunchType::SteamAppInfo,

                "--skip-boot" => launch_profile.skip_boot = true,
                "--skip-title" => launch_profile.skip_title = true,
                "--render-physics" => launch_profile.render_physics = true,
                "--debug" => {
                    launch_profile.skip_boot = true;
                    launch_profile.skip_title = true;
                    launch_profile.render_physics = true;
                }
                _ => not_changed_count += 1,
            }
        }
        launch_profile.changed = not_changed_count != args.len() - 1;

        launch_profile
    }
}
