use bevy::asset::{AssetServer, Handle, LoadState};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Event, Clone, Copy)]
pub struct LoadAssetGroup {
    pub group: &'static str,
    pub images: &'static [(u32, &'static str)],
    pub fonts: &'static [(u32, &'static str)],
}

#[derive(Event, Clone, Copy)]
pub struct AssetGroupLoaded(pub &'static str);

#[derive(Resource, Default)]
pub struct AssetStore {
    images: HashMap<u32, Handle<Image>>,
    fonts: HashMap<u32, Handle<Font>>,
    sounds: HashMap<u32, Handle<AudioSource>>,
}

impl AssetStore {
    pub fn image<K: Into<u32>>(&self, key: K) -> Option<Handle<Image>> {
        self.images.get(&key.into()).cloned()
    }
    pub fn font<K: Into<u32>>(&self, key: K) -> Option<Handle<Font>> {
        self.fonts.get(&key.into()).cloned()
    }
    pub fn sound<K: Into<u32>>(&self, key: K) -> Option<Handle<AudioSource>> {
        self.sounds.get(&key.into()).cloned()
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
            .add_systems(Update, handle_load_requests)
            .add_systems(
                Update,
                poll_pending_groups
                    .run_if(pending_not_empty)
                    .run_if(on_timer(Duration::from_millis(50))),
            );
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
            store.images.insert(*key, h);
        }
        for (key, path) in req.fonts {
            let h: Handle<Font> = server.load(*path);
            list.push(h.clone().untyped());
            store.fonts.insert(*key, h);
        }

        pending.inner.insert(req.group, list);
    }
}

fn pending_not_empty(pending: Res<PendingGroups>) -> bool {
    !pending.inner.is_empty()
}

fn poll_pending_groups(
    server: Res<AssetServer>,
    mut pending: ResMut<PendingGroups>,
    mut done_writer: EventWriter<AssetGroupLoaded>,
) {
    info!("in poll");

    pending.inner.retain(|group, handles| {
        handles.retain(|h| !matches!(server.get_load_state(h.id()), Some(LoadState::Loaded)));

        if handles.is_empty() {
            done_writer.write(AssetGroupLoaded(*group));
            false
        } else {
            true
        }
    });
}
