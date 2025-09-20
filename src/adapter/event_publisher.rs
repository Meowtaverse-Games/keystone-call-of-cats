use bevy::prelude::*;

use crate::core::boundary::{EventPublisher, GameEvent};

#[derive(Event)]
pub struct BevyGameEvent(pub GameEvent);

pub struct BevyEventPublisher<'w>(pub EventWriter<'w, BevyGameEvent>);

impl<'w> EventPublisher for BevyEventPublisher<'w> {
    fn publish(&mut self, event: GameEvent) {
        self.0.write(BevyGameEvent(event));
    }
}
