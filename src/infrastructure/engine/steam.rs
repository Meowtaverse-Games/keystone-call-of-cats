use bevy::prelude::*;

/// Thin wrapper Resource to expose bevy_steamworks::Client as SteamClient for existing systems.
#[derive(Resource, Clone)]
pub struct SteamClient(pub bevy_steamworks::Client);

pub fn provide_steam_client(mut commands: Commands, client: Res<bevy_steamworks::Client>) {
    commands.insert_resource(SteamClient(client.clone()));
}
