use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::resources::game_state::GameState;

pub mod components;
pub mod systems;

pub use systems::StageProgressionState;

pub struct StageScenePlugin;
impl Plugin for StageScenePlugin {
    fn build(&self, app: &mut App) {
        // SystemSetの順序関係を設定
        systems::StageSystemSet::configure_sets(app);

        app.init_resource::<systems::StageProgressionState>()
            .add_message::<systems::StoneCommandMessage>()
            .add_message::<systems::StoneAppendCommandMessage>()
            .add_systems(OnEnter(GameState::Stage), systems::setup)
            .add_systems(
                OnEnter(GameState::Stage),
                crate::systems::engine::friction::apply_zero_friction_to_rigid_bodies
                    .after(systems::setup),
            )
            .add_systems(OnExit(GameState::Stage), systems::cleanup)
            // Input: メッセージの受信、UI入力
            .add_systems(
                Update,
                (
                    systems::handle_stone_messages,
                    systems::handle_stone_append_messages,
                    systems::handle_tutorial_overlay_input,
                )
                    .in_set(systems::StageSystemSet::Input)
                    .run_if(in_state(GameState::Stage)),
            )
            // Script: スクリプト実行
            .add_systems(
                Update,
                systems::tick_script_program
                    .in_set(systems::StageSystemSet::Script)
                    .run_if(in_state(GameState::Stage)),
            )
            // Reset: リセット処理
            .add_systems(
                Update,
                (
                    systems::restore_dug_tiles,
                    systems::despawn_placed_tiles,
                    systems::reset_stone_position,
                    systems::reset_player_position,
                )
                    .chain()
                    .in_set(systems::StageSystemSet::Reset)
                    .run_if(in_state(GameState::Stage)),
            )
            // Animation: アニメーション更新
            .add_systems(
                Update,
                (systems::animate_player, systems::animate_obstacle)
                    .in_set(systems::StageSystemSet::Animation)
                    .run_if(in_state(GameState::Stage)),
            )
            // Movement: 移動処理
            .add_systems(
                Update,
                (
                    systems::move_player,
                    systems::update_stone_behavior,
                    systems::update_stage_root,
                )
                    .in_set(systems::StageSystemSet::Movement)
                    .run_if(in_state(GameState::Stage)),
            )
            // Collision: 衝突検出・処理
            .add_systems(
                Update,
                systems::carry_riders_with_stone
                    .in_set(systems::StageSystemSet::Collision)
                    .run_if(in_state(GameState::Stage)),
            )
            // Goal: ゴール判定
            .add_systems(
                Update,
                (
                    systems::check_goal_completion,
                    systems::drive_player_goal_descent,
                )
                    .chain()
                    .in_set(systems::StageSystemSet::Goal)
                    .run_if(in_state(GameState::Stage)),
            )
            // Progression: ステージ進行処理
            .add_systems(
                Update,
                (
                    systems::advance_stage_if_cleared,
                    systems::reload_stage_if_needed,
                )
                    .chain()
                    .in_set(systems::StageSystemSet::Progression)
                    .run_if(in_state(GameState::Stage)),
            )
            // Audio: 音声処理（Movementの後、他のシステムと並列実行可能）
            .add_systems(
                Update,
                systems::update_stage_color_grading
                    .in_set(systems::StageSystemSet::Audio)
                    .run_if(in_state(GameState::Stage)),
            )
            // UI: UI更新（最後に実行）
            .add_systems(
                EguiPrimaryContextPass,
                systems::ui
                    .in_set(systems::StageSystemSet::UI)
                    .run_if(in_state(GameState::Stage)),
            );
    }
}
