use crate::plugins::assets_loader::LoadAssetGroup;

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum ImageKey {
    Logo,
    Spa,
    PlayerIdle1,
    PlayerIdle2,
    PlayerIdle3,
    PlayerIdle4,
    PlayerRun1,
    PlayerRun2,
    PlayerRun3,
    PlayerRun4,
    PlayerRun5,
    PlayerRun6,
    PlayerRun7,
    PlayerRun8,
    PlayerRun9,
    PlayerRun10,
}

impl From<ImageKey> for u32 {
    fn from(v: ImageKey) -> u32 {
        v as u32
    }
}

pub const PLAYER_IDLE_KEYS: [ImageKey; 4] = [
    ImageKey::PlayerIdle1,
    ImageKey::PlayerIdle2,
    ImageKey::PlayerIdle3,
    ImageKey::PlayerIdle4,
];

pub const PLAYER_RUN_KEYS: [ImageKey; 10] = [
    ImageKey::PlayerRun1,
    ImageKey::PlayerRun2,
    ImageKey::PlayerRun3,
    ImageKey::PlayerRun4,
    ImageKey::PlayerRun5,
    ImageKey::PlayerRun6,
    ImageKey::PlayerRun7,
    ImageKey::PlayerRun8,
    ImageKey::PlayerRun9,
    ImageKey::PlayerRun10,
];

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
    images: &[
        (ImageKey::Logo as u32, "images/logo_with_black.png"),
        (
            ImageKey::PlayerIdle1 as u32,
            "images/SPA/Player/Iddle/1.png",
        ),
        (
            ImageKey::PlayerIdle2 as u32,
            "images/SPA/Player/Iddle/2.png",
        ),
        (
            ImageKey::PlayerIdle3 as u32,
            "images/SPA/Player/Iddle/3.png",
        ),
        (
            ImageKey::PlayerIdle4 as u32,
            "images/SPA/Player/Iddle/4.png",
        ),
        (ImageKey::PlayerRun1 as u32, "images/SPA/Player/Run/1.png"),
        (ImageKey::PlayerRun2 as u32, "images/SPA/Player/Run/2.png"),
        (ImageKey::PlayerRun3 as u32, "images/SPA/Player/Run/3.png"),
        (ImageKey::PlayerRun4 as u32, "images/SPA/Player/Run/4.png"),
        (ImageKey::PlayerRun5 as u32, "images/SPA/Player/Run/5.png"),
        (ImageKey::PlayerRun6 as u32, "images/SPA/Player/Run/6.png"),
        (ImageKey::PlayerRun7 as u32, "images/SPA/Player/Run/7.png"),
        (ImageKey::PlayerRun8 as u32, "images/SPA/Player/Run/8.png"),
        (ImageKey::PlayerRun9 as u32, "images/SPA/Player/Run/9.png"),
        (ImageKey::PlayerRun10 as u32, "images/SPA/Player/Run/10.png"),
    ],
    fonts: &[
        (FontKey::Default as u32, "fonts/PixelMplus12-Regular.ttf"),
        (FontKey::Title as u32, "fonts/Quicky Story.ttf"),
    ],
};
