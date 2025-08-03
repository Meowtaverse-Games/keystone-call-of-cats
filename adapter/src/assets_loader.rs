use bevy::asset::{AssetServer, Handle, LoadState, LoadedUntypedAsset};
use bevy::prelude::*;

use crate::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AssetGroup {
    Splash,
    Game,
}

struct AssetsLoadedEvent(pub AssetGroup);

/// 単一グループのハンドル集合
#[derive(Default)]
pub struct LoadingGroup {
    pub handles: Vec<Handle<LoadedUntypedAsset>>,
}

#[derive(Resource, Default)]
struct LoadingGroups {
    pub splash: LoadingGroup,
    pub game: LoadingGroup,
}

impl LoadingGroup {
    /// 指定パスをロードし、Untyped ハンドルを蓄積
    pub fn load(&mut self, asset_server: Res<AssetServer>, path: &str) {
        let handle = asset_server.load_untyped(path);
        self.handles.push(handle);
    }

    /// 全ハンドルが Loaded かどうかをチェック
    pub fn is_loaded(&self, asset_server: &Res<AssetServer>) -> bool {
        self.handles.iter().for_each(|handle| {
            if matches!(
                asset_server.get_load_state(handle.id()),
                Some(LoadState::Loaded)
            ) {
                warn!("Asset not loaded: {:?}", handle.id());
            }
        });
        return false
            == self.handles.iter().any(|handle| {
                matches!(
                    asset_server.get_load_state(handle.id()),
                    Some(LoadState::Loaded)
                )
            });
    }
}

/// AssetLoader プラグイン本体
pub struct AssetsLoaderPlugin;

#[derive(Event)]
pub struct SplashAssetsLoadedEvent;

#[derive(Event)]
pub struct GameAssetsLoadedEvent;

impl Plugin for AssetsLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingGroups>()
            .add_event::<SplashAssetsLoadedEvent>()
            .add_event::<GameAssetsLoadedEvent>()
            .add_systems(Update, load_splash_assets.run_if(in_state(GameState::Boot)));
        // .add_system(load_game_assets .in_schedule(OnEnter(AppState::Next)))

        // 毎フレーム読み込み完了をチェックし、該当イベントを発行
        // .add_system(check_and_fire_events);
    }
}

fn load_splash_assets(mut loading: ResMut<LoadingGroups>, asset_server: Res<AssetServer>) {
    let lg = &mut loading.splash;
    lg.load(asset_server, "splash.png");
}

fn load_game_assets(mut loading: ResMut<LoadingGroups>, asset_server: Res<AssetServer>) {
    let lg = &mut loading.game;
    // // ロゴやUI画像
    // lg.load(asset_server, "images/logo.png");
    // lg.load(asset_server, "images/player_sprite.png");
    // // フォント
    // lg.load(asset_server, "fonts/FiraSans-Bold.ttf");
    // lg.load(asset_server, "fonts/FiraSans-Regular.ttf");
    // // BGMやSE（必要なら）
    // // lg.load(&asset_server, "audio/bgm.ogg");
    // // lg.load(&asset_server, "audio/jump.wav");
}

/// 毎フレーム呼ばれるシステム: 読み込み完了を検知してイベント発行
fn check_and_fire_events(
    loading: Res<LoadingGroups>,
    asset_server: Res<AssetServer>,
    mut splash_ev: EventWriter<SplashAssetsLoadedEvent>,
    mut game_ev: EventWriter<GameAssetsLoadedEvent>,
) {
    // スプラッシュ用アセットが全部 Loaded なら
    if loading.splash.is_loaded(&asset_server) {
        splash_ev.send(SplashAssetsLoadedEvent);
    }
    // 本編用アセットが全部 Loaded なら
    if loading.game.is_loaded(&asset_server) {
        game_ev.send(GameAssetsLoadedEvent);
    }
}
