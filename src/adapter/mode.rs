use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct Mode {
    pub changed: bool,
    pub skip_boot: bool,
    pub skip_title: bool,
    pub render_physics: bool,
}

impl Mode {
    pub fn from_args(args: &[String]) -> Self {
        let mut mode = Self::default();
        let mut not_change_count = 0;

        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--skip-boot" => mode.skip_boot = true,
                "--skip-title" => mode.skip_title = true,
                "--render-physics" => mode.render_physics = true,
                "--debug" => {
                    mode.skip_boot = true;
                    mode.skip_title = true;
                    mode.render_physics = true;
                }
                _ => not_change_count += 1,
            }
        }
        mode.changed = not_change_count != args.len() - 1;

        mode
    }
}
