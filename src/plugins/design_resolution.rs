use bevy::{
    camera::ScalingMode,
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};

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

#[derive(Resource, Copy, Clone)]
struct MaskColor(Color);

#[derive(Resource, Copy, Clone)]
struct VirtualResolution {
    width: f32,
    height: f32,
}

#[derive(Resource, Copy, Clone, Default, Debug)]
pub struct LetterboxOffsets {
    pub left: f32,
    pub right: f32,
}

#[derive(Component)]
struct HudRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MaskSide {
    Left,
    Right,
    Top,
    Bottom,
}

pub struct DesignResolutionPlugin {
    pub design: DesignResolution,
    pub min_width: f32,
    pub min_height: f32,
    pub mask_color: Color,
}

impl DesignResolutionPlugin {
    pub fn new(width: f32, height: f32, mask_color: Color) -> Self {
        Self {
            design: DesignResolution::new(width, height),
            min_width: width,
            min_height: height,
            mask_color,
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
        app.insert_resource(VirtualResolution {
            width: self.design.width,
            height: self.design.height,
        })
        .insert_resource(AutoMinConfig {
            min_width: self.min_width,
            min_height: self.min_height,
        })
        .insert_resource(MaskColor(self.mask_color))
        .insert_resource(LetterboxOffsets::default())
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, setup_ui_root)
        .add_systems(Update, update_letterbox);
    }
}

fn setup_camera(mut commands: Commands, config: Res<AutoMinConfig>) {
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

fn setup_ui_root(
    mut commands: Commands,
    design: Res<VirtualResolution>,
    mask_color: Res<MaskColor>,
) {
    let parent = commands
        .spawn((
            Name::new("Design Resolution Root"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();

    let hud = commands
        .spawn((
            Name::new("UI Root"),
            HudRoot,
            Node {
                width: Val::Px(design.width),
                height: Val::Px(design.height),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();

    commands.entity(parent).add_child(hud);

    let color = mask_color.0;

    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Name::new("Letterbox Mask Left"),
            MaskSide::Left,
            Node {
                width: Val::Px(0.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
        parent.spawn((
            Name::new("Letterbox Mask Right"),
            MaskSide::Right,
            Node {
                width: Val::Px(0.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
        parent.spawn((
            Name::new("Letterbox Mask Top"),
            MaskSide::Top,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
        parent.spawn((
            Name::new("Letterbox Mask Bottom"),
            MaskSide::Bottom,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
    });

    commands.insert_resource(UIRoot(hud));
}

fn update_letterbox(
    mut first_run: Local<bool>,
    mut resize_events: EventReader<WindowResized>,
    windows: Query<&Window, With<PrimaryWindow>>,
    design: Res<VirtualResolution>,
    mut ui_scale: ResMut<UiScale>,
    offsets: Res<LetterboxOffsets>,
    mut hud_and_masks: ParamSet<(
        Query<&mut Node, With<HudRoot>>,
        Query<(&MaskSide, &mut Node), With<MaskSide>>,
    )>,
) {
    let mut should_update = *first_run || offsets.is_changed();
    for _ in resize_events.read() {
        should_update = true;
    }

    if !should_update {
        return;
    }

    *first_run = false;

    let Ok(window) = windows.single() else {
        return;
    };

    let window_size = window.resolution.size();
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return;
    }

    let left_offset = offsets.left.max(0.0);
    let right_offset = offsets.right.max(0.0);

    let available_width = (window_size.x - left_offset - right_offset).max(0.0);
    if design.width <= 0.0 || design.height <= 0.0 {
        return;
    }

    let scale_x = available_width / design.width;
    let scale_y = window_size.y / design.height;

    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return;
    }

    let min_scale = scale_x.min(scale_y);
    if !min_scale.is_finite() || min_scale <= 0.0 {
        return;
    }

    ui_scale.0 = min_scale;

    let horizontal_overflow = if scale_x > min_scale {
        design.width * (scale_x / min_scale - 1.0)
    } else {
        0.0
    };
    let horizontal_overflow = horizontal_overflow.max(0.0);

    let vertical_overflow = if scale_y > min_scale {
        design.height * (scale_y / min_scale - 1.0)
    } else {
        0.0
    };
    let vertical_overflow = vertical_overflow.max(0.0);

    let left_margin = left_offset / min_scale + horizontal_overflow / 2.0;
    let right_margin = right_offset / min_scale + horizontal_overflow / 2.0;

    let stage_top = vertical_overflow / 2.0;
    let stage_bottom = stage_top + design.height;
    let stage_left = left_margin;
    let stage_right = stage_left + design.width;
    let total_width = stage_left + design.width + right_margin;
    let total_height = design.height + vertical_overflow;

    if let Ok(mut node) = hud_and_masks.p0().single_mut() {
        node.margin.left = Val::Px(stage_left);
        node.margin.top = Val::Px(stage_top);
    }

    for (side, mut node) in hud_and_masks.p1().iter_mut() {
        match side {
            MaskSide::Left => {
                node.width = Val::Px(stage_left.max(0.0));
                node.left = Val::Px(0.0);
                node.top = Val::Px(0.0);
                node.height = Val::Px(total_height.max(0.0));
            }
            MaskSide::Right => {
                node.width = Val::Px(right_margin.max(0.0));
                node.left = Val::Px(stage_right);
                node.top = Val::Px(0.0);
                node.height = Val::Px(total_height.max(0.0));
            }
            MaskSide::Top => {
                node.height = Val::Px(stage_top.max(0.0));
                node.top = Val::Px(0.0);
                node.width = Val::Px(total_width.max(0.0));
                node.left = Val::Px(0.0);
            }
            MaskSide::Bottom => {
                let bottom_height = (total_height - stage_bottom).max(0.0);
                node.height = Val::Px(bottom_height);
                node.top = Val::Px(stage_bottom);
                node.width = Val::Px(total_width.max(0.0));
                node.left = Val::Px(0.0);
            }
        }
    }
}
