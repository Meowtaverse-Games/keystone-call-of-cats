use bevy::prelude::*;
use bevy::asset::LoadState;
use crate::scenes::AppState;               // シーン層で定義している AppState
use crate::core::{                            // コア層で定義したイベント
    SplashAssetsLoadedEvent,
    GameAssetsLoadedEvent,
    AssetGroup,
};

/// 各ロード・グループのハンドルを保持するリソース
#[derive(Resource, Default)]
pub struct LoadingGroups {
    pub splash: LoadingGroup,
    pub game:   LoadingGroup,
}

/// 単一グループのハンドル集合
#[derive(Default)]
pub struct LoadingGroup {
    pub handles: Vec<HandleUntyped>,
}

impl LoadingGroup {
    /// 指定パスをロードし、Untyped ハンドルを蓄積
    pub fn load(&mut self, asset_server: &AssetServer, path: &str) {
        let handle = asset_server.load(path).clone_untyped();
        self.handles.push(handle);
    }

    /// 全ハンドルが Loaded かどうかをチェック
    pub fn is_loaded(&self, asset_server: &AssetServer) -> bool {
        let states = asset_server
            .get_load_states(self.handles.iter().map(|h| h.id()));
        states.iter().all(|&s| s == LoadState::Loaded)
    }
}

/// AssetLoader プラグイン本体
pub struct AssetsLoaderPlugin;

impl Plugin for AssetsLoaderPlugin {
    fn build(&self, app: &mut App) {
        app
            // リソースとイベントの初期化
            .init_resource::<LoadingGroups>()
            .add_event::<SplashAssetsLoadedEvent>()
            .add_event::<GameAssetsLoadedEvent>()

            // スプラッシュ用アセット読み込み開始
            .add_system(load_splash_assets.in_schedule(OnEnter(AppState::Loading)))
            // 本編用アセット読み込み開始（例として Loading ステート脱出後に）
            .add_system(load_game_assets .in_schedule(OnEnter(AppState::Next)))

            // 毎フレーム読み込み完了をチェックし、該当イベントを発行
            .add_system(check_and_fire_events);
    }
}

/// OnEnter(AppState::Loading): スプラッシュ画面用アセットを読み込み
fn load_splash_assets(
    mut loading: ResMut<LoadingGroups>,
    asset_server: Res<AssetServer>,
) {
    let lg = &mut loading.splash;
    lg.load(&asset_server, "splash.png");
    // 必要ならさらにスプラッシュに使う画像やフォントを追加
    // lg.load(&asset_server, "fonts/LoadingFont.ttf");
}

/// OnEnter(AppState::Next): ゲーム本編用アセットを読み込み
fn load_game_assets(
    mut loading: ResMut<LoadingGroups>,
    asset_server: Res<AssetServer>,
) {
    let lg = &mut loading.game;
    // ロゴやUI画像
    lg.load(&asset_server, "images/logo.png");
    lg.load(&asset_server, "images/player_sprite.png");
    // フォント
    lg.load(&asset_server, "fonts/FiraSans-Bold.ttf");
    lg.load(&asset_server, "fonts/FiraSans-Regular.ttf");
    // BGMやSE（必要なら）
    // lg.load(&asset_server, "audio/bgm.ogg");
    // lg.load(&asset_server, "audio/jump.wav");
}

/// 毎フレーム呼ばれるシステム: 読み込み完了を検知してイベント発行
fn check_and_fire_events(
    loading: Res<LoadingGroups>,
    asset_server: Res<AssetServer>,
    mut splash_ev: EventWriter<SplashAssetsLoadedEvent>,
    mut game_ev:   EventWriter<GameAssetsLoadedEvent>,
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
