use avian2d::prelude::*;
use bevy::prelude::*;

pub fn apply_zero_friction_to_rigid_bodies(
    mut commands: Commands,
    query: Query<Entity, (With<RigidBody>, Without<Friction>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(Friction::ZERO);
    }
}
