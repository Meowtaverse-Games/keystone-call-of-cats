use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::{resources::game_state::GameState, scenes::stage::systems::tick_pending_tutorial};

pub mod components;
pub mod systems;

pub use systems::StageProgressionState;

/// Only normal play owns stage selection, reloads and persistence. Headless and
/// replay register the same lifecycle systems but use their own seeded drivers.
#[derive(Resource, Default)]
struct GraphicalStageLifecycle;

pub struct StageScenePlugin;
impl Plugin for StageScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraphicalStageLifecycle>()
            .add_plugins((StageGameplayPlugin, StagePresentationPlugin))
            .add_systems(
                Update,
                tick_pending_tutorial.run_if(in_state(GameState::Stage)),
            )
            .add_systems(
                Update,
                systems::handle_tutorial_overlay_input
                    .in_set(systems::StageSystemSet::Input)
                    .run_if(in_state(GameState::Stage)),
            );
    }
}

fn register_stage_lifecycle(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::Stage),
        systems::setup.run_if(resource_exists::<GraphicalStageLifecycle>),
    )
    .add_systems(
        OnEnter(GameState::Stage),
        crate::resources::external_control::invalidate_external_generation
            .run_if(resource_exists::<crate::resources::external_control::ExternalControlState>),
    )
    .add_systems(
        OnEnter(GameState::Stage),
        crate::systems::engine::friction::apply_zero_friction_to_rigid_bodies
            .after(systems::setup)
            .run_if(resource_exists::<GraphicalStageLifecycle>),
    )
    .add_systems(
        OnExit(GameState::Stage),
        (
            systems::cleanup.run_if(resource_exists::<GraphicalStageLifecycle>),
            crate::resources::external_control::invalidate_external_generation.run_if(
                resource_exists::<crate::resources::external_control::ExternalControlState>,
            ),
        ),
    )
    .add_systems(
        Update,
        crate::resources::external_control::drain_external_commands
            .in_set(systems::StageSystemSet::Input)
            .run_if(resource_exists::<crate::resources::external_control::ExternalControlBridge>)
            .run_if(in_state(GameState::Stage)),
    )
    .add_systems(
        Update,
        crate::resources::external_control::publish_external_snapshot
            .after(systems::update_stone_behavior)
            .run_if(resource_exists::<crate::resources::external_control::ExternalControlBridge>)
            .run_if(in_state(GameState::Stage)),
    )
    .add_systems(
        Update,
        systems::update_stage_root
            .run_if(resource_exists::<GraphicalStageLifecycle>)
            .in_set(systems::StageSystemSet::Movement)
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
            .run_if(resource_exists::<GraphicalStageLifecycle>)
            .in_set(systems::StageSystemSet::Progression)
            .run_if(in_state(GameState::Stage)),
    );
}

/// Product gameplay registration shared by graphical play and headless verification.
pub struct StageGameplayPlugin;
impl Plugin for StageGameplayPlugin {
    fn build(&self, app: &mut App) {
        systems::StageSystemSet::configure_sets(app);
        app.insert_resource(crate::resources::player_physics::PlayerPhysicsParams::embedded());
        register_stage_lifecycle(app);
        app.init_resource::<systems::StageProgressionState>()
            .add_message::<systems::StoneCommandMessage>()
            .add_message::<systems::StoneAppendCommandMessage>()
            .add_message::<systems::StoneExternalOutcomeMessage>()
            .add_message::<systems::StoneTickMessage>()
            .add_systems(
                Update,
                systems::rescale_stage_motion
                    .after(crate::systems::engine::design_resolution::update_letterbox)
                    .before(systems::StageSystemSet::Movement)
                    .run_if(in_state(GameState::Stage)),
            )
            .add_message::<systems::StonePlaceRequestMessage>()
            // Input: メッセージの受信、UI入力
            .add_systems(
                Update,
                (
                    systems::handle_stone_messages,
                    systems::handle_stone_append_messages,
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
                    systems::reset_placed_tiles,
                    systems::restore_dug_tiles,
                    systems::reset_stone_position,
                    systems::reset_player_position,
                )
                    .chain()
                    .in_set(systems::StageSystemSet::Reset)
                    .run_if(in_state(GameState::Stage)),
            )
            // Obstacle animation also changes colliders and is gameplay, not presentation.
            .add_systems(
                Update,
                systems::animate_obstacle
                    .in_set(systems::StageSystemSet::Animation)
                    .run_if(in_state(GameState::Stage)),
            )
            // Movement: 移動処理
            .add_systems(
                Update,
                (
                    systems::move_player,
                    systems::update_stone_behavior,
                    systems::resolve_place_requests.after(systems::update_stone_behavior),
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
            );
    }
}

/// Identical physics and timed terrain registration for both launch configurations.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimedTerrain;

pub struct StagePhysicsPlugin;
impl Plugin for StagePhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(avian2d::prelude::PhysicsPlugins::default())
            .add_systems(Update, systems::update_dynamic_solid.in_set(TimedTerrain));
    }
}

/// The real stage animation, color grading and egui UI, also used by replay.
pub struct StagePresentationPlugin;
impl Plugin for StagePresentationPlugin {
    fn build(&self, app: &mut App) {
        app
            // Animation: アニメーション更新
            .add_systems(
                Update,
                systems::animate_player
                    .in_set(systems::StageSystemSet::Animation)
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
