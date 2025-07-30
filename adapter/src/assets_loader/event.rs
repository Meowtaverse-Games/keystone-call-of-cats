#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetGroup {
    Splash,
    Game,
}

pub struct AssetsLoadedEvent(pub AssetGroup);
