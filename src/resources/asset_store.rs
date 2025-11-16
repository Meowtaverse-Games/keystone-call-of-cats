use bevy::{
    asset::{Handle, UntypedHandle},
    prelude::{AudioSource, Font, Image, Message, Resource},
};
use std::collections::HashMap;

#[derive(Message, Clone, Copy)]
pub struct LoadAssetGroup {
    pub group: &'static str,
    pub images: &'static [(u32, &'static str)],
    pub fonts: &'static [(u32, &'static str)],
    pub audio: &'static [(u32, &'static str)],
}

#[derive(Message, Clone, Copy)]
pub struct AssetGroupLoaded;

#[derive(Resource, Default)]
pub struct AssetStore {
    images: HashMap<u32, Handle<Image>>,
    fonts: HashMap<u32, Handle<Font>>,
    audio: HashMap<u32, Handle<AudioSource>>,
}

impl AssetStore {
    pub fn image<K: Into<u32>>(&self, key: K) -> Option<Handle<Image>> {
        self.images.get(&key.into()).cloned()
    }

    pub fn font<K: Into<u32>>(&self, key: K) -> Option<Handle<Font>> {
        self.fonts.get(&key.into()).cloned()
    }

    pub fn audio<K: Into<u32>>(&self, key: K) -> Option<Handle<AudioSource>> {
        self.audio.get(&key.into()).cloned()
    }

    pub(crate) fn insert_image(&mut self, key: u32, handle: Handle<Image>) {
        self.images.insert(key, handle);
    }

    pub(crate) fn insert_font(&mut self, key: u32, handle: Handle<Font>) {
        self.fonts.insert(key, handle);
    }

    pub(crate) fn insert_audio(&mut self, key: u32, handle: Handle<AudioSource>) {
        self.audio.insert(key, handle);
    }
}

#[derive(Resource, Default)]
pub struct PendingGroups {
    pub inner: HashMap<&'static str, Vec<UntypedHandle>>,
}
