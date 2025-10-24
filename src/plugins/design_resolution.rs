use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};

#[derive(Resource, Copy, Clone)]
struct MaskColor(Color);

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

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MaskSide {
    Left,
    Right,
    Top,
    Bottom,
}

pub struct DesignResolutionPlugin {
    pub design_resolution_size: Vec2,
    pub mask_color: Color,
}

impl DesignResolutionPlugin {
    pub fn new(width: f32, height: f32, mask_color: Color) -> Self {
        Self {
            design_resolution_size: Vec2::new(width, height),
            mask_color,
        }
    }
}

impl Plugin for DesignResolutionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MaskColor(self.mask_color))
            .insert_resource(LetterboxOffsets::default())
            .insert_resource(ScaledViewport {
                center: self.design_resolution_size / 2.0,
                size: self.design_resolution_size,
                scale: 1.0,
            })
            .add_systems(Startup, setup)
            .add_systems(Update, update_letterbox);
    }
}

fn setup(mut commands: Commands, mask_color: Res<MaskColor>) {
    let color = mask_color.0;

    let parent = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },))
        .id();

    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            MaskSide::Left,
            Node {
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));

        parent.spawn((
            MaskSide::Right,
            Node {
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
        parent.spawn((
            MaskSide::Top,
            Node {
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
        parent.spawn((
            MaskSide::Bottom,
            Node {
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
        ));
    });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_letterbox(
    mut first_run: Local<bool>,
    mut resize_events: MessageReader<WindowResized>,
    windows: Query<&Window, With<PrimaryWindow>>,
    offsets: Res<LetterboxOffsets>,
    mut scaled_viewport: ResMut<ScaledViewport>,
    mut mask_sides: Query<(&MaskSide, &mut Node), With<MaskSide>>,
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

    let window_size = Vec2::new(window.resolution.width(), window.resolution.height());

    let left_offset = offsets.left.max(0.0);
    let right_offset = offsets.right.max(0.0);

    let available_width = (window_size.x - left_offset - right_offset).max(0.0);
    let available_height = window_size.y;

    let scale_x = available_width / scaled_viewport.size.x;
    let scale_y = window_size.y / scaled_viewport.size.y;

    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return;
    }

    let scale_min = scale_x.min(scale_y);

    let width = scaled_viewport.size.x * scale_min;
    let height = scaled_viewport.size.y * scale_min;

    let horizontal_overflow = if scale_x > scale_min {
        available_width - width
    } else {
        0.0
    };
    let horizontal_overflow = horizontal_overflow.max(0.0);

    let vertical_overflow = if scale_y > scale_min {
        available_height - height
    } else {
        0.0
    };
    let vertical_overflow = vertical_overflow.max(0.0);

    let left_margin = left_offset + horizontal_overflow / 2.0;
    let right_margin = right_offset + horizontal_overflow / 2.0;

    let content_top = vertical_overflow / 2.0;
    let content_bottom = content_top + height;
    let content_left = left_margin;
    let content_right = content_left + width;

    for (side, mut node) in mask_sides.iter_mut() {
        match side {
            MaskSide::Left => {
                node.width = Val::Px(content_left.max(0.0));
                node.height = Val::Px(available_height.max(0.0));
            }
            MaskSide::Right => {
                node.left = Val::Px(content_right);
                node.width = Val::Px(right_margin.max(0.0));
                node.height = Val::Px(available_height.max(0.0));
            }
            MaskSide::Top => {
                node.height = Val::Px(content_top.max(0.0));
                node.width = Val::Px(available_width.max(0.0));
            }
            MaskSide::Bottom => {
                let bottom_height = (available_height - content_bottom).max(0.0);

                node.top = Val::Px(content_bottom);
                node.height = Val::Px(bottom_height);
                node.width = Val::Px(available_width.max(0.0));
            }
        }
    }

    let new_viewport = ScaledViewport {
        center: Vec2::new(content_left + width / 2.0, content_top + height / 2.0),
        size: scaled_viewport.size,
        scale: scale_min,
    };
    if scaled_viewport.center != new_viewport.center || scaled_viewport.scale != new_viewport.scale
    {
        *scaled_viewport = new_viewport;
    }
    info!(
        "Updated scaled viewport: center={:?}, size={:?}, scale={}",
        scaled_viewport.center, scaled_viewport.size, scaled_viewport.scale
    );
}
