// 各シーンのサブモジュールを公開
pub mod title;
//pub mod gameplay;
//pub mod gameover;

// まとめてプラグインを登録するヘルパー関数
use bevy::prelude::*;
use title::TitlePlugin;
// use gameplay::GameplayPlugin;
// use gameover::GameOverPlugin;

pub fn register_scenes(app: &mut App) -> &mut App {
    app.add_plugins((
        TitlePlugin,
        // GameplayPlugin, GameOverPlugin
    ))
}
