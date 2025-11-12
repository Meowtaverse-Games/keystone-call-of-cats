use bevy::prelude::*;

use crate::resources::steam_client::SteamClientResource;

pub fn pump_steam_callbacks_system(client: Res<SteamClientResource>) {
    client.client.run_callbacks();
}
