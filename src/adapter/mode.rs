use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy)]
pub struct Mode {
    pub changed: bool,
    pub show_boot_screen: bool,
    pub render_physics: bool,
}

impl Mode {
    pub fn from_args(args: &[String]) -> Self {
        let mut mode = Self::default();
        let mut not_change_count = 0;

        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--skip-boot" => mode.show_boot_screen = false,
                "--render-physics" => mode.render_physics = true,
                "--debug" => {
                    mode.show_boot_screen = false;
                    mode.render_physics = true;
                }
                _ => not_change_count += 1,
            }
        }
        mode.changed = not_change_count != args.len() - 1;

        mode
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            changed: false,
            show_boot_screen: true,
            render_physics: false,
        }
    }
}
