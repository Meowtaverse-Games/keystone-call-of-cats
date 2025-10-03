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
        app.insert_resource(VirtualResolution {
            width: self.design.width,
            height: self.design.height,
        })
        .insert_resource(AutoMinConfig {
            min_width: self.min_width,
            min_height: self.min_height,
        })
        .insert_resource(MaskColor(self.mask_color))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, setup_ui_root);
        // .add_systems(Update, update_letterbox);
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
    mut hud_and_masks: ParamSet<(
        Query<&mut Node, With<HudRoot>>,
        Query<(&MaskSide, &mut Node), With<MaskSide>>,
    )>,
) {
    let mut should_update = *first_run;
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

    let scale_x = window_size.x / design.width;
    let scale_y = window_size.y / design.height;
    let min_scale = scale_x.min(scale_y);

    ui_scale.0 = min_scale;

    let mut horizontal_overflow = if scale_x > min_scale {
        design.width * (scale_x / min_scale - 1.0)
    } else {
        0.0
    };

    if horizontal_overflow < 0.0 {
        horizontal_overflow = 0.0;
    }

    let mut vertical_overflow = if scale_y > min_scale {
        design.height * (scale_y / min_scale - 1.0)
    } else {
        0.0
    };

    if vertical_overflow < 0.0 {
        vertical_overflow = 0.0;
    }

    if let Ok(mut node) = hud_and_masks.p0().single_mut() {
        node.margin.left = Val::Px(horizontal_overflow / 2.0);
        node.margin.top = Val::Px(vertical_overflow / 2.0);
    }

    for (side, mut node) in hud_and_masks.p1().iter_mut() {
        match side {
            MaskSide::Left => {
                node.width = Val::Px(horizontal_overflow);
                node.left = Val::Px(-horizontal_overflow / 2.0);
                node.top = Val::Px(-vertical_overflow / 2.0);
                node.height = Val::Px(design.height + vertical_overflow);
            }
            MaskSide::Right => {
                node.width = Val::Px(horizontal_overflow);
                node.left = Val::Px(design.width + horizontal_overflow / 2.0);
                node.top = Val::Px(-vertical_overflow / 2.0);
                node.height = Val::Px(design.height + vertical_overflow);
            }
            MaskSide::Top => {
                node.height = Val::Px(vertical_overflow);
                node.top = Val::Px(-vertical_overflow / 2.0);
                node.width = Val::Px(design.width + horizontal_overflow);
                node.left = Val::Px(-horizontal_overflow / 2.0);
            }
            MaskSide::Bottom => {
                node.height = Val::Px(vertical_overflow);
                node.top = Val::Px(design.height + vertical_overflow / 2.0);
                node.width = Val::Px(design.width + horizontal_overflow);
                node.left = Val::Px(-horizontal_overflow / 2.0);
            }
        }
    }
}
