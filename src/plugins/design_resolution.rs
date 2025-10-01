use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy_aspect_ratio_mask::{AspectRatioMask, AspectRatioPlugin, Hud, Resolution};

use crate::core::domain::graphics::design_resolution::DesignResolution;

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

#[derive(Resource, Copy, Clone, Debug)]
pub struct UIRoot(pub Entity);

#[derive(Resource, Copy, Clone)]
struct AutoMinConfig {
    min_width: f32,
    min_height: f32,
}

pub struct DesignResolutionPlugin {
    pub design: DesignResolution,
    pub min_width: f32,
    pub min_height: f32,
    pub mask_color: Color,
}

impl DesignResolutionPlugin {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            design: DesignResolution::new(width, height),
            min_width: width,
            min_height: height,
            mask_color: Color::BLACK,
        }
    }

    pub fn fix_min(mut self, width: f32, height: f32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }
}

impl Plugin for DesignResolutionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Resolution {
            width: self.design.width,
            height: self.design.height,
        })
        .insert_resource(AspectRatioMask {
            color: self.mask_color,
        })
        .insert_resource(AutoMinConfig {
            min_width: self.min_width,
            min_height: self.min_height,
        })
        .add_plugins(AspectRatioPlugin::default())
        .add_systems(Startup, (setup, capture_hud_root));
    }
}

fn setup(mut commands: Commands, config: Res<AutoMinConfig>) {
    commands.spawn((
        MainCamera,
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: config.min_width,
                min_height: config.min_height,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn capture_hud_root(mut commands: Commands, hud: Res<Hud>) {
    commands.insert_resource(UIRoot(hud.0));
}
