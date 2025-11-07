use bevy::prelude::*;

#[derive(Component)]
pub struct StageSelectRoot;

#[derive(Component)]
pub struct StageCard {
    pub index: usize,
}

#[derive(Component)]
pub struct StagePlayButton {
    pub stage_index: usize,
    pub enabled: bool,
}

#[derive(Component)]
pub struct StagePageButton {
    pub delta: isize,
}

#[derive(Component)]
pub struct StagePageIndicator;

#[derive(Component)]
pub struct StageBackButton;

#[derive(Component, Clone, Copy)]
pub struct ButtonVisual {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub disabled: Color,
    pub enabled: bool,
}

impl ButtonVisual {
    pub fn new(
        normal: Color,
        hovered: Color,
        pressed: Color,
        disabled: Color,
        enabled: bool,
    ) -> Self {
        Self {
            normal,
            hovered,
            pressed,
            disabled,
            enabled,
        }
    }

    pub fn color_for(&self, interaction: &Interaction) -> Color {
        if !self.enabled {
            return self.disabled;
        }

        match interaction {
            Interaction::Pressed => self.pressed,
            Interaction::Hovered => self.hovered,
            Interaction::None => self.normal,
        }
    }
}
