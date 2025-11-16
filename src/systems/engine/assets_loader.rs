use bevy::{
    asset::{AssetServer, LoadState, UntypedHandle},
    prelude::*,
};

use crate::resources::asset_store::{AssetGroupLoaded, AssetStore, LoadAssetGroup, PendingGroups};

pub fn handle_load_requests(
    mut reader: MessageReader<LoadAssetGroup>,
    server: Res<AssetServer>,
    mut store: ResMut<AssetStore>,
    mut pending: ResMut<PendingGroups>,
) {
    for req in reader.read().copied() {
        let mut handles: Vec<UntypedHandle> = Vec::new();

        for (key, path) in req.images {
            let handle: Handle<Image> = server.load(*path);
            handles.push(handle.clone().untyped());
            store.insert_image(*key, handle);
        }
        for (key, path) in req.fonts {
            let handle: Handle<Font> = server.load(*path);
            handles.push(handle.clone().untyped());
            store.insert_font(*key, handle);
        }
        for (key, path) in req.audio {
            let handle: Handle<AudioSource> = server.load(*path);
            handles.push(handle.clone().untyped());
            store.insert_audio(*key, handle);
        }

        pending.inner.insert(req.group, handles);
    }
}

pub fn pending_not_empty(pending: Res<PendingGroups>) -> bool {
    !pending.inner.is_empty()
}

pub fn poll_pending_groups(
    server: Res<AssetServer>,
    mut pending: ResMut<PendingGroups>,
    mut writer: MessageWriter<AssetGroupLoaded>,
) {
    pending.inner.retain(|_, handles| {
        handles.retain(|handle| {
            !matches!(server.get_load_state(handle.id()), Some(LoadState::Loaded))
        });

        if handles.is_empty() {
            writer.write(AssetGroupLoaded);
            false
        } else {
            true
        }
    });
}
