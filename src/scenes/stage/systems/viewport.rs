use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{resources::design_resolution::ScaledViewport, scenes::stage::components::*};

use super::StoneCommandState;
use crate::resources::player_physics::PlayerPhysicsParams;

/// Run before the root adopts the new scale and before movement writes velocities.
pub fn rescale_stage_motion(
    viewport: Res<ScaledViewport>,
    physics: Res<PlayerPhysicsParams>,
    roots: Query<(Entity, &Transform), With<StageRoot>>,
    mut velocities: Query<(&ChildOf, &mut LinearVelocity)>,
    mut players: Query<(&ChildOf, &mut GravityScale, Option<&mut PlayerGoalDescent>), With<Player>>,
    mut stones: Query<(&ChildOf, &mut StoneCommandState)>,
) {
    for (root, transform) in &roots {
        let ratio = viewport.scale / transform.scale.x;
        if ratio != 1.0 {
            for (parent, mut velocity) in &mut velocities {
                if parent.parent() == root {
                    velocity.0 *= ratio;
                }
            }
            for (parent, mut state) in &mut stones {
                if parent.parent() == root {
                    state.rescale_world_distance(ratio);
                }
            }
        }
        for (parent, mut gravity, descent) in &mut players {
            if parent.parent() != root {
                continue;
            }
            if let Some(mut descent) = descent {
                // Descent disables gravity, but its saved value is in world units.
                if ratio != 1.0 {
                    descent.original_gravity *= ratio;
                }
            } else {
                let target = physics.gravity_scale * viewport.scale;
                if gravity.0 != target {
                    gravity.0 = target;
                }
            }
        }
    }
}
