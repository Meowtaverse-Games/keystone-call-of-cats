use std::io::*;

use bevy::prelude::*;
use steamworks::{self, AppId, Client as SteamworksClient, PersonaStateChange};

#[derive(Resource, Clone)]
pub struct SteamClient(SteamworksClient);

impl SteamClient {
    pub fn remote_storage(&self) -> steamworks::RemoteStorage {
        self.0.remote_storage()
    }
}

pub struct SteamPlugin {
    client: SteamworksClient,
}

impl SteamPlugin {
    pub fn new(app_id: impl Into<AppId>) -> Self {
        let client =
            SteamworksClient::init_app(app_id).expect("Failed to initialize Steamworks SDK");

        Self { client }
    }
}

impl Plugin for SteamPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SteamClient(self.client.clone()));
    }
}

pub fn show_steam_app_info(app_id: u32) {
    let client = SteamworksClient::init_app(app_id).unwrap();

    let _cb = client.register_callback(|p: PersonaStateChange| {
        println!("Got callback: {:?}", p);
    });

    let utils = client.utils();
    println!("Utils:");
    println!("AppId: {:?}", utils.app_id());
    println!("UI Language: {}", utils.ui_language());

    let apps = client.apps();
    println!("Apps");
    println!("IsInstalled: {}", apps.is_app_installed(AppId(app_id)));
    println!("InstallDir: {}", apps.app_install_dir(AppId(app_id)));
    println!("BuildId: {}", apps.app_build_id());
    println!("AppOwner: {:?}", apps.app_owner());
    println!("Langs: {:?}", apps.available_game_languages());
    println!("Lang: {}", apps.current_game_language());
    println!("Beta: {:?}", apps.current_beta_name());

    println!("Subscribed to this app: {}", apps.is_subscribed());
    println!(
        "Subscribed to app_id({}): {}",
        app_id,
        apps.is_subscribed_app(AppId(app_id))
    );

    let rs = client.remote_storage();

    println!(
        "cloud_enabled_app={} cloud_enabled_user={}",
        rs.is_cloud_enabled_for_app(),
        rs.is_cloud_enabled_for_account()
    );

    rs.files().iter().for_each(|f| {
        println!("File: {:?} (size: {:?})", f.name, f.size,);
    });

    let file = steamworks::RemoteStorage::file(&rs, "test.txt");
    let mut writer = file.write();

    if let Err(e) = writer.write_all(b"Hello, Steam Remote Storage!") {
        eprintln!("Failed to write to Steam Remote Storage: {}", e);
    }
}
