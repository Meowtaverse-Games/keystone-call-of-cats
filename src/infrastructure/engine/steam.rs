use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct SteamClient(bevy_steamworks::Client);

pub fn provide_steam_client(mut commands: Commands, client: Res<bevy_steamworks::Client>) {
    commands.insert_resource(SteamClient(client.clone()));
}
