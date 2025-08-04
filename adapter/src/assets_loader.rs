use bevy::asset::{AssetServer, Handle, LoadState};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AssetsLoaderState {
    #[default]
    Loading,
    Done,
}

#[derive(Event)]
pub struct AssetsLoadedEvent;

#[derive(Default)]
pub struct ImageAssets {
    pub handles: HashMap<String, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct Assets {
    images: ImageAssets,
}

impl Assets {
    pub fn load_image(&mut self, asset_server: Res<AssetServer>, key: &str, path: &str) {
        let handle: Handle<Image> = asset_server.load(path);
        self.images.handles.insert(key.to_string(), handle);
    }

    pub fn is_loaded(&self, asset_server: &Res<AssetServer>) -> bool {
        self.images
            .handles
            .values()
            .for_each(|handle| match asset_server.get_load_state(handle) {
                Some(LoadState::Loaded) => {}
                _ => warn!("Asset not loaded: {:?}", handle.id()),
            });
        return self.images.handles.values().any(|handle| {
            match asset_server.get_load_state(handle) {
                Some(LoadState::Loaded) => true,
                _ => false,
            }
        });
    }
}

pub struct AssetsLoaderPlugin;

impl Plugin for AssetsLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets>()
            .init_state::<AssetsLoaderState>()
            .add_event::<AssetsLoadedEvent>()
            .add_systems(OnEnter(AssetsLoaderState::Loading), load_splash_assets)
            .add_systems(
                Update,
                check_and_fire_events.run_if(in_state(AssetsLoaderState::Loading)),
            );
    }
}

fn load_splash_assets(mut assets: ResMut<Assets>, asset_server: Res<AssetServer>) {
    assets.load_image(asset_server, "logo", "images/logo_with_black.png");
}

/// 毎フレーム呼ばれるシステム: 読み込み完了を検知してイベント発行
fn check_and_fire_events(
    assets: Res<Assets>,
    asset_server: Res<AssetServer>,
    mut assets_loaded_ev: EventWriter<AssetsLoadedEvent>,
    mut assets_loader_state: ResMut<NextState<AssetsLoaderState>>,
) {
    if assets.is_loaded(&asset_server) {
        print!("Assets loaded successfully.");
        assets_loaded_ev.write(AssetsLoadedEvent);
        assets_loader_state.set(AssetsLoaderState::Done);
    }
}
