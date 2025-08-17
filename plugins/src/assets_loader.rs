use bevy::asset::{AssetServer, Handle, LoadState};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Event, Clone, Copy)]
pub struct LoadAssetGroup {
    pub group: &'static str,
    pub images: &'static [(&'static u32, &'static str)],
    pub fonts: &'static [(&'static u32, &'static str)],
}

#[derive(Event, Clone, Copy)]
pub struct AssetGroupLoaded(pub &'static str);

#[derive(Resource, Default)]
pub struct AssetStore {
    images: HashMap<&'static u32, Handle<Image>>,
    fonts: HashMap<&'static u32, Handle<Font>>,
    sounds: HashMap<&'static u32, Handle<AudioSource>>,
}

impl AssetStore {
    pub fn image(&self, key: &'static u32) -> Option<Handle<Image>> {
        self.images.get(key).cloned()
    }
    pub fn font(&self, key: &'static u32) -> Option<Handle<Font>> {
        self.fonts.get(key).cloned()
    }
    pub fn sound(&self, key: &'static u32) -> Option<Handle<AudioSource>> {
        self.sounds.get(key).cloned()
    }
}

#[derive(Resource, Default)]
struct PendingGroups {
    inner: HashMap<&'static str, Vec<UntypedHandle>>,
}

pub struct AssetLoaderPlugin;
impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetStore>()
            .init_resource::<PendingGroups>()
            .add_event::<LoadAssetGroup>()
            .add_event::<AssetGroupLoaded>()
            .add_systems(Update, (handle_load_requests, poll_pending_groups));
    }
}

fn handle_load_requests(
    mut ev: EventReader<LoadAssetGroup>,
    server: Res<AssetServer>,
    mut store: ResMut<AssetStore>,
    mut pending: ResMut<PendingGroups>,
) {
    for req in ev.read().copied() {
        let mut list: Vec<UntypedHandle> = Vec::new();

        for (key, path) in req.images {
            let h: Handle<Image> = server.load(*path);
            list.push(h.clone().untyped());
            store.images.insert(key, h);
        }
        for (key, path) in req.fonts {
            let h: Handle<Font> = server.load(*path);
            list.push(h.clone().untyped());
            store.fonts.insert(key, h);
        }

        pending.inner.insert(req.group, list);
    }
}

fn poll_pending_groups(
    server: Res<AssetServer>,
    mut pending: ResMut<PendingGroups>,
    mut done_writer: EventWriter<AssetGroupLoaded>,
) {
    let keys: Vec<&'static str> = pending.inner.keys().copied().collect();
    for group in keys {
        let all_loaded = pending.inner[&group]
            .iter()
            .all(|u| matches!(server.get_load_state(u.id()), Some(LoadState::Loaded)));
        if all_loaded {
            pending.inner.remove(&group);
            done_writer.write(AssetGroupLoaded(group));
        }
    }
}
