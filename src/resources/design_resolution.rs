use bevy::prelude::*;

#[derive(Resource, Copy, Clone)]
pub struct MaskColor(pub Color);

/// Controls whether the letterbox mask UI is visible.
#[derive(Resource, Copy, Clone, Debug)]
pub struct LetterboxVisibility(pub bool);

#[derive(Resource, Copy, Clone, Default, Debug)]
pub struct LetterboxOffsets {
    pub left: f32,
    pub right: f32,
}

#[derive(Resource, Copy, Clone, Debug)]
pub struct ScaledViewport {
    pub center: Vec2,
    pub size: Vec2,
    pub scale: f32,
}

impl ScaledViewport {
    pub fn new(size: Vec2) -> Self {
        Self {
            center: size / 2.0,
            size,
            scale: 1.0,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum MaskSide {
    Left,
    Right,
    Top,
    Bottom,
}
