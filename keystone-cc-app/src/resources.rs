use bevy::prelude::*;
use keystone_cc_core::boundary::{ScoreRepo, EventPublisher};

#[derive(Resource)]
pub struct ScoreResource(pub Box<dyn ScoreRepo>);

#[derive(Resource)]
pub struct EventPublisherResource(pub Box<dyn EventPublisher>);
