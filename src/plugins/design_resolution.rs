use bevy::prelude::*;

use crate::{
    resources::design_resolution::{
        LetterboxOffsets, LetterboxVisibility, MaskColor, ScaledViewport,
    },
    systems::engine::design_resolution::{spawn_letterbox_masks, update_letterbox},
};

pub struct DesignResolutionPlugin {
    desired_size: Vec2,
    mask_color: Color,
}

impl DesignResolutionPlugin {
    pub fn new(width: f32, height: f32, mask_color: Color) -> Self {
        Self {
            desired_size: Vec2::new(width, height),
            mask_color,
        }
    }
}

impl Plugin for DesignResolutionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MaskColor(self.mask_color))
            .insert_resource(LetterboxVisibility(true))
            .insert_resource(LetterboxOffsets::default())
            .insert_resource(ScaledViewport::new(self.desired_size))
            .add_systems(Startup, spawn_letterbox_masks)
            .add_systems(Update, update_letterbox);
    }
}
