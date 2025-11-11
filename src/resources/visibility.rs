use bevy::prelude::States;

/// Tracks when the primary window becomes visible.
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum VisibilityState {
    #[default]
    Hidden,
    Shown,
}
