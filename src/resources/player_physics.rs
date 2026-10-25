//! Player values in their original, unscaled product units.
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerPhysicsParams {
    pub gravity_scale: f32,
    pub speed: f32,
    pub jump_speed: f32,
    pub capsule_radius: f32,
    pub capsule_length: f32,
    pub capsule_center_y: f32,
    pub ground_foot_offset: f32,
    pub ground_cast_width: f32,
    pub ground_cast_height: f32,
    pub ground_cast_distance: f32,
    pub goal_descent_speed: f32,
    pub goal_align_x: f32,
    pub spawn_y_offset: f32,
    /// Update count, not a duration: retain the existing goal descent timing.
    pub goal_descent_frames: u32,
}

impl Default for PlayerPhysicsParams {
    fn default() -> Self {
        Self {
            gravity_scale: 80.0,
            speed: 120.0,
            jump_speed: 380.0,
            capsule_radius: 1.4,
            capsule_length: 1.2,
            capsule_center_y: -0.6,
            ground_foot_offset: 12.3,
            ground_cast_width: 8.0,
            ground_cast_height: 0.4,
            ground_cast_distance: 3.0,
            goal_descent_speed: 32.0,
            goal_align_x: 4.0,
            spawn_y_offset: 2.2,
            goal_descent_frames: 280,
        }
    }
}
impl PlayerPhysicsParams {
    pub fn embedded() -> Self {
        let params: Self = ron::from_str(include_str!("../../assets/config/player_physics.ron"))
            .expect("Invalid embedded player physics RON");
        params
            .validate()
            .expect("Invalid embedded player physics values");
        params
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.gravity_scale.is_finite() || self.gravity_scale <= 0.0 {
            return Err("player_physics.gravity_scale must be finite and positive".into());
        }
        if !self.speed.is_finite() || self.speed <= 0.0 {
            return Err("player_physics.speed must be finite and positive".into());
        }
        if !self.jump_speed.is_finite() || self.jump_speed <= 0.0 {
            return Err("player_physics.jump_speed must be finite and positive".into());
        }
        if !self.capsule_radius.is_finite() || self.capsule_radius <= 0.0 {
            return Err("player_physics.capsule_radius must be finite and positive".into());
        }
        if !self.capsule_length.is_finite() || self.capsule_length <= 0.0 {
            return Err("player_physics.capsule_length must be finite and positive".into());
        }
        if !self.capsule_center_y.is_finite() {
            return Err("player_physics.capsule_center_y must be finite".into());
        }
        if !self.ground_foot_offset.is_finite() || self.ground_foot_offset <= 0.0 {
            return Err("player_physics.ground_foot_offset must be finite and positive".into());
        }
        if !self.ground_cast_width.is_finite() || self.ground_cast_width <= 0.0 {
            return Err("player_physics.ground_cast_width must be finite and positive".into());
        }
        if !self.ground_cast_height.is_finite() || self.ground_cast_height <= 0.0 {
            return Err("player_physics.ground_cast_height must be finite and positive".into());
        }
        if !self.ground_cast_distance.is_finite() || self.ground_cast_distance <= 0.0 {
            return Err("player_physics.ground_cast_distance must be finite and positive".into());
        }
        if !self.goal_descent_speed.is_finite() || self.goal_descent_speed <= 0.0 {
            return Err("player_physics.goal_descent_speed must be finite and positive".into());
        }
        if !self.goal_align_x.is_finite() {
            return Err("player_physics.goal_align_x must be finite".into());
        }
        if !self.spawn_y_offset.is_finite() {
            return Err("player_physics.spawn_y_offset must be finite".into());
        }
        if self.goal_descent_frames == 0 {
            return Err("player_physics.goal_descent_frames must be positive".into());
        }
        Ok(())
    }
}

/// Omitted fields inherit the embedded product configuration, not zero values.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerPhysicsOverrides {
    pub gravity_scale: Option<f32>,
    pub speed: Option<f32>,
    pub jump_speed: Option<f32>,
    pub capsule_radius: Option<f32>,
    pub capsule_length: Option<f32>,
    pub capsule_center_y: Option<f32>,
    pub ground_foot_offset: Option<f32>,
    pub ground_cast_width: Option<f32>,
    pub ground_cast_height: Option<f32>,
    pub ground_cast_distance: Option<f32>,
    pub goal_descent_speed: Option<f32>,
    pub goal_align_x: Option<f32>,
    pub spawn_y_offset: Option<f32>,
    pub goal_descent_frames: Option<u32>,
}
impl PlayerPhysicsOverrides {
    pub fn resolve(&self) -> Result<PlayerPhysicsParams, String> {
        let mut params = PlayerPhysicsParams::embedded();
        if let Some(value) = self.gravity_scale {
            params.gravity_scale = value;
        }
        if let Some(value) = self.speed {
            params.speed = value;
        }
        if let Some(value) = self.jump_speed {
            params.jump_speed = value;
        }
        if let Some(value) = self.capsule_radius {
            params.capsule_radius = value;
        }
        if let Some(value) = self.capsule_length {
            params.capsule_length = value;
        }
        if let Some(value) = self.capsule_center_y {
            params.capsule_center_y = value;
        }
        if let Some(value) = self.ground_foot_offset {
            params.ground_foot_offset = value;
        }
        if let Some(value) = self.ground_cast_width {
            params.ground_cast_width = value;
        }
        if let Some(value) = self.ground_cast_height {
            params.ground_cast_height = value;
        }
        if let Some(value) = self.ground_cast_distance {
            params.ground_cast_distance = value;
        }
        if let Some(value) = self.goal_descent_speed {
            params.goal_descent_speed = value;
        }
        if let Some(value) = self.goal_align_x {
            params.goal_align_x = value;
        }
        if let Some(value) = self.spawn_y_offset {
            params.spawn_y_offset = value;
        }
        if let Some(value) = self.goal_descent_frames {
            params.goal_descent_frames = value;
        }
        params.validate()?;
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_ron_matches_original_defaults() {
        assert_eq!(
            PlayerPhysicsParams::embedded(),
            PlayerPhysicsParams::default()
        );
    }

    #[test]
    fn partial_overrides_preserve_other_embedded_values_and_reject_invalid_values() {
        let overrides: PlayerPhysicsOverrides =
            serde_json::from_str(r#"{"gravity_scale":88.0}"#).unwrap();
        assert_eq!(
            overrides.resolve().unwrap(),
            PlayerPhysicsParams {
                gravity_scale: 88.0,
                ..PlayerPhysicsParams::embedded()
            }
        );
        for invalid in [-1.0, 0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                PlayerPhysicsOverrides {
                    gravity_scale: Some(invalid),
                    ..Default::default()
                }
                .resolve()
                .is_err()
            );
        }
        assert!(
            PlayerPhysicsOverrides {
                capsule_center_y: Some(f32::NAN),
                ..Default::default()
            }
            .resolve()
            .is_err()
        );
        assert!(
            PlayerPhysicsOverrides {
                goal_descent_frames: Some(0),
                ..Default::default()
            }
            .resolve()
            .is_err()
        );
        assert!(serde_json::from_str::<PlayerPhysicsOverrides>(r#"{"unknown":1}"#).is_err());
    }
}
