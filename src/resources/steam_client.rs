use bevy::prelude::Resource;
use steamworks::{Client, RemoteStorage};

#[derive(Resource, Clone)]
pub struct SteamClientResource {
    pub client: Client,
}

impl SteamClientResource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn remote_storage(&self) -> RemoteStorage {
        self.client.remote_storage()
    }
}
