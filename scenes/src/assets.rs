use keystone_cc_plugins::assets_loader::LoadAssetGroup;

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum ImageKey {
    Logo,
}

impl From<ImageKey> for u32 {
    fn from(v: ImageKey) -> u32 {
        v as u32
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum FontKey {
    Default,
    Title,
}

impl From<FontKey> for u32 {
    fn from(v: FontKey) -> u32 {
        v as u32
    }
}

pub const DEFAULT_GROUP: LoadAssetGroup = LoadAssetGroup {
    group: "default",
    images: &[(ImageKey::Logo as u32, "images/logo_with_black.png")],
    fonts: &[
        (FontKey::Default as u32, "fonts/PixelMplus12-Regular.ttf"),
        (FontKey::Title as u32, "fonts/Quicky Story.ttf"),
    ],
};
