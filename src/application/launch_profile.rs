use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LaunchProfile {
    pub changed: bool,
    pub skip_boot: bool,
    pub skip_title: bool,
    pub render_physics: bool,
}

impl LaunchProfile {
    pub fn from_args(args: &[String]) -> Self {
        let mut launch_profile = Self::default();
        let mut not_change_count = 0;

        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--skip-boot" => launch_profile.skip_boot = true,
                "--skip-title" => launch_profile.skip_title = true,
                "--render-physics" => launch_profile.render_physics = true,
                "--debug" => {
                    launch_profile.skip_boot = true;
                    launch_profile.skip_title = true;
                    launch_profile.render_physics = true;
                }
                _ => not_change_count += 1,
            }
        }
        launch_profile.changed = not_change_count != args.len() - 1;

        launch_profile
    }
}
