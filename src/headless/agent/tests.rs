use super::*;

fn default_reach() -> &'static AgentReach {
    static REACH: std::sync::OnceLock<AgentReach> = std::sync::OnceLock::new();
    REACH.get_or_init(|| reach::measure_agent(1.0, &PlayerPhysicsParams::embedded()).unwrap())
}

fn scenario(name: &str, code: Option<&str>) -> Scenario {
    let mut value = serde_json::json!({
        "ron_path": format!("examples/headless/fixtures/agent-{name}.ron"),
        "seed_start": 17, "seed_count": 1, "max_frames": 1800
    });
    if let Some(code) = code {
        value["programs"] = serde_json::json!([{"stone_index":0,"language":"Rhai","code":code}]);
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn physical_routes_and_exported_inputs_reproduce() {
    let reach = default_reach();
    assert_eq!(reach.max_gap(), 2);
    assert_eq!(reach.jump_headroom, 2);
    assert!(!reach.takeoffs(false, true, 1).is_empty());
    assert!(reach.takeoffs(false, true, 2).is_empty());
    for (name, code, expected, max_attempts) in [
        ("walk", None, "cleared", 1),
        ("step", None, "cleared", 1),
        ("gap", None, "cleared", 2),
        ("high", None, "unsolved", 1),
        ("high", Some("place(\"up\");"), "cleared", 2),
        ("move", Some("move(\"right\");"), "cleared", 1),
    ] {
        let result = solve(&scenario(name, code), 17, reach, 8).unwrap();
        println!(
            "{name} {code:?}: {} frames={} failures={}",
            result.status,
            result.frames,
            serde_json::to_string(&result.failures).unwrap()
        );
        assert_eq!(result.status, expected, "{name} {code:?}");
        assert!(result.attempts <= max_attempts, "{name} {code:?}");
        if let Some(solution) = result.solution {
            let exported: Scenario =
                serde_json::from_str(&serde_json::to_string(&solution).unwrap()).unwrap();
            let verified = verify(&exported, 1).unwrap();
            assert!(verified.seeds[0].cleared);
            assert_eq!(
                verified.seeds[0].trajectory_hash,
                result.result.unwrap().trajectory_hash
            );
        }
    }
}

#[test]
fn stage_fixture_varies_and_successes_replay() {
    let reach = default_reach();
    let scenario: Scenario =
        serde_json::from_str(&fs::read_to_string("examples/headless/agent-stage-1.json").unwrap())
            .unwrap();
    let mut terrain = BTreeSet::new();
    let mut cleared = 0;
    for seed in 17..21 {
        let result = solve(&scenario, seed, reach, 8).unwrap();
        println!(
            "stage1 seed {seed}: {} attempts {}",
            result.status, result.attempts
        );
        assert_eq!(result.status, "cleared");
        assert!(result.attempts <= 2);
        terrain.insert(result.terrain_hash.unwrap());
        if let Some(solution) = result.solution {
            cleared += 1;
            assert_eq!(
                verify(&solution, 1).unwrap().seeds[0].trajectory_hash,
                result.result.unwrap().trajectory_hash
            );
        }
    }
    assert!(terrain.len() > 1);
    assert_eq!(cleared, 4);
}

#[test]
fn low_ceiling_corridor_waits_for_the_stone_and_replays() {
    let scenario: Scenario =
        serde_json::from_str(&fs::read_to_string("examples/headless/agent-corridor.json").unwrap())
            .unwrap();
    let result = solve(&scenario, 17, default_reach(), 8).unwrap();
    assert_eq!(result.status, "cleared");
    assert_eq!(result.attempts, 1);
    assert!(result.failures.is_empty());
    assert!(result.runs[0].planned_wait_until > scenario.start_frame + 30);
    assert!(
        result.runs[0]
            .route
            .iter()
            .all(|segment| matches!(segment.action, Action::Walk | Action::Drop))
    );
    let exported: Scenario =
        serde_json::from_str(&serde_json::to_string(&result.solution.unwrap()).unwrap()).unwrap();
    assert!(!exported.seed_inputs[&17].is_empty());
    let verified = verify(&exported, 1).unwrap();
    assert!(verified.seeds[0].cleared);
    assert_eq!(
        verified.seeds[0].trajectory_hash,
        result.result.unwrap().trajectory_hash
    );

    let no_code: Scenario = serde_json::from_str(
        &fs::read_to_string("examples/headless/agent-corridor-no-code.json").unwrap(),
    )
    .unwrap();
    let result = solve(&no_code, 17, default_reach(), 8).unwrap();
    assert_eq!(result.status, "unsolved");
    assert_eq!(result.attempts, 1);
    assert!(result.solution.is_none());
}

#[test]
fn invalid_inputs_and_budgets_are_rejected_before_measurement() {
    let mut scenario = scenario("walk", None);
    assert!(run(&scenario, 1, 0).is_err());
    assert!(run(&scenario, 0, 8).is_err());
    scenario.inputs.push(InputEvent {
        frame: 1,
        key: InputKey::Space,
        pressed: true,
    });
    assert!(run(&scenario, 1, 8).is_err());
}

#[test]
fn script_errors_are_not_unsolved() {
    let reach = AgentReach {
        trials: vec![],
        jump_headroom: 0,
    };
    for code in ["this is not valid rhai", "loop {}"] {
        let result = solve(&scenario("high", Some(code)), 17, &reach, 8).unwrap();
        assert_eq!(result.status, "error");
        assert!(result.reason.is_some());
        assert!(result.solution.is_none());
    }
}

#[test]
fn measurement_follows_overridden_physics() {
    let mut scenario = scenario("step", None);
    scenario.player_physics.jump_speed = Some(100.0);
    let measured = reach::measure_agent(
        scenario.viewport_scale,
        &scenario.player_physics.resolve().unwrap(),
    )
    .unwrap();
    assert!(measured.takeoffs(false, true, 1).is_empty());
    assert!(measured.jump_headroom < default_reach().jump_headroom);
    let result = solve(&scenario, 17, &measured, 2).unwrap();
    assert_eq!(result.status, "unsolved");
    assert!(result.solution.is_none());
}

#[test]
fn goal_tail_is_completed_and_observations_do_not_change_replay() {
    let mut scenario = scenario("walk", None);
    scenario.post_goal_frames = 40;
    let result = solve(&scenario, 17, default_reach(), 2).unwrap();
    assert_eq!(result.status, "cleared");
    let physical = result.result.unwrap();
    assert_eq!(physical.frames, physical.goal_frame.unwrap() + 41);
    assert_eq!(
        verify(&result.solution.unwrap(), 1).unwrap().seeds[0].trajectory_hash,
        physical.trajectory_hash
    );
}

#[test]
fn future_stone_arrangement_requires_a_timed_wait() {
    let scenario = scenario("move", Some("sleep(5); move(\"right\");"));
    let result = solve(&scenario, 17, default_reach(), 8).unwrap();
    assert_eq!(
        result.status,
        "cleared",
        "{}",
        serde_json::to_string(&result.failures).unwrap()
    );
    assert!(
        result
            .runs
            .iter()
            .any(|attempt| attempt.planned_wait_until >= 300)
    );
    let replay = verify(&result.solution.unwrap(), 1).unwrap();
    assert_eq!(
        replay.seeds[0].trajectory_hash,
        result.result.unwrap().trajectory_hash
    );
}

#[test]
fn retries_start_fresh_and_respect_both_budgets() {
    let mut scenario = scenario("walk", None);
    scenario.max_frames = 40;
    let result = solve(&scenario, 17, default_reach(), 2).unwrap();
    assert_eq!(result.status, "unsolved");
    assert_eq!(result.attempts, 2);
    assert_eq!(result.frames, 80);
    assert_eq!(result.failures.len(), 2);
    for failure in &result.failures {
        assert_eq!(failure.frame, 40);
        assert!(failure.segment.is_some());
        assert!(
            failure.player_cell[0] < 7.0,
            "Each attempt restarts at the original spawn"
        );
    }
    assert_ne!(
        result.failures[0].segment.as_ref().unwrap().to,
        result.failures[1].segment.as_ref().unwrap().to
    );
    assert!(result.solution.is_none());
}

#[test]
fn passages_check_headroom_and_the_complete_landing_column_in_both_directions() {
    let grid = Grid::new("........\n........\n........\n........\n........\n........");
    for direction in [-1, 1] {
        let from = [3, 2];
        let to = [3 + direction * 3, 1];
        assert!(grid.passage_clear(from, to, 2));
        // Departure ceiling, intermediate ceiling, and the landing column's
        // departure height all used to be omitted by gap-jump checks.
        for blocker in [
            [3, 3],
            [3, 4],
            [3 + direction, 3],
            [3 + direction * 2, 4],
            [to[0], 2],
            [to[0], 3],
            to,
        ] {
            let mut blocked = grid.clone();
            blocked.solid.insert(blocker);
            assert!(
                !blocked.passage_clear(from, to, 2),
                "direction={direction} blocker={blocker:?}"
            );
        }
        let drop = [3 + direction, 0];
        assert!(grid.passage_clear(from, drop, 0));
        for y in 0..=from[1] {
            let mut blocked = grid.clone();
            blocked.solid.insert([drop[0], y]);
            assert!(!blocked.passage_clear(from, drop, 0));
        }
        // A low ceiling can prevent jumping without preventing walking/dropping.
        let mut ceiling = grid.clone();
        ceiling.solid.insert([3, 3]);
        assert!(ceiling.passage_clear(from, [3 + direction, 2], 0));
        assert!(ceiling.passage_clear(from, drop, 0));

        let step = [3 + direction, 3];
        let mut supported = grid.clone();
        supported.solid.insert([step[0], 2]);
        assert!(supported.passage_clear(from, step, 2));
        supported.solid.insert([step[0], 4]);
        assert!(!supported.passage_clear(from, step, 2));
    }
    assert!(!grid.passage_clear([3, 4], [5, 4], 2));
}

#[test]
fn passage_clearance_uses_observed_stones_platforms_and_digging() {
    let base = Grid::new("......\n......\n......\n......\n......");
    let from = [1, 1];
    let to = [4, 1];
    let obstacle = PositionSample {
        cell: [2, 3],
        world_px: [0.0; 2],
    };
    let mut state = SensedState {
        frame: 30,
        goal_reached: false,
        player_cell: [1.5, 1.5],
        velocity: [0.0; 2],
        grounded: true,
        on_stone: None,
        scene: TrajectoryPoint {
            stones: vec![],
            placed_tiles: vec![],
            dug_tiles: vec![],
            frame: 30,
            cell: from,
            world_px: [0.0; 2],
        },
    };
    assert!(base.observed(&state).passage_clear(from, to, 2));
    state.scene.stones.push(StoneSample {
        stone_index: 0,
        position: obstacle.clone(),
    });
    assert!(!base.observed(&state).passage_clear(from, to, 2));
    state.scene.stones.clear();
    state.scene.placed_tiles.push(obstacle.clone());
    assert!(!base.observed(&state).passage_clear(from, to, 2));
    state.scene.placed_tiles.clear();
    let mut terrain = base.clone();
    terrain.solid.insert(obstacle.cell);
    assert!(!terrain.observed(&state).passage_clear(from, to, 2));
    state.scene.dug_tiles.push(obstacle);
    assert!(terrain.observed(&state).passage_clear(from, to, 2));
}
