#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DesignResolution {
    pub width: f32,
    pub height: f32,
}

impl DesignResolution {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}
