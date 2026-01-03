use bevy::{asset::LoadedFolder, prelude::*};

#[derive(Resource)]
pub struct LocaleFolder(pub Handle<LoadedFolder>);
