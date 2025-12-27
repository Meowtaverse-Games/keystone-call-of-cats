use bevy::prelude::*;

/// ステージシーンのシステム実行順序を定義するSystemSet
///
/// 実行順序:
/// 1. Input - 入力処理（メッセージの受信など）
/// 2. Script - スクリプト実行
/// 3. Reset - リセット処理（プレイヤー・石の位置リセット）
/// 4. Animation - アニメーション更新
/// 5. Movement - 移動処理
/// 6. Collision - 衝突検出・処理
/// 7. Goal - ゴール判定
/// 8. Progression - ステージ進行処理
/// 9. Audio - 音声処理
/// 10. UI - UI更新
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StageSystemSet {
    /// 入力処理（メッセージの受信、UI入力など）
    Input,
    /// スクリプト実行
    Script,
    /// リセット処理（プレイヤー・石の位置リセット）
    Reset,
    /// アニメーション更新
    Animation,
    /// 移動処理
    Movement,
    /// 衝突検出・処理
    Collision,
    /// ゴール判定
    Goal,
    /// ステージ進行処理（クリア判定、ステージ進行など）
    Progression,
    /// 音声処理
    Audio,
    /// UI更新
    UI,
}

impl StageSystemSet {
    /// SystemSet間の順序関係を設定
    pub fn configure_sets(app: &mut App) {
        app.configure_sets(
            Update,
            (
                StageSystemSet::Input,
                StageSystemSet::Script.after(StageSystemSet::Input),
                StageSystemSet::Reset.after(StageSystemSet::Script),
                StageSystemSet::Animation.after(StageSystemSet::Reset),
                StageSystemSet::Movement.after(StageSystemSet::Animation),
                StageSystemSet::Collision.after(StageSystemSet::Movement),
                StageSystemSet::Goal.after(StageSystemSet::Collision),
                StageSystemSet::Progression.after(StageSystemSet::Goal),
                // AudioはMovementの後、他のシステムと並列実行可能
                StageSystemSet::Audio.after(StageSystemSet::Movement),
                // UIは最後に実行（Progressionの後）
                StageSystemSet::UI.after(StageSystemSet::Progression),
            ),
        );
    }
}
