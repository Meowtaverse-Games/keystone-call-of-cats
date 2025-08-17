use keystone_cc_plugins::assets_loader::LoadAssetGroup;

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum Images {
    Logo,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum Font {
    Default,
    Title,
}

pub const DEFAULT_GROUP: LoadAssetGroup = LoadAssetGroup {
    group: "default",
    images: &[(&(Images::Logo as u32), "images/logo_with_black.png")],
    fonts: &[
        (&(Font::Default as u32), "fonts/PixelMplus12-Regular.ttf"),
        (&(Font::Title as u32), "fonts/Quicky Story.ttf"),
    ],
};
