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
    assert!(!reach.takeoffs(false, true, 1).is_empty());
    assert!(reach.takeoffs(false, true, 2).is_empty());
    for (name, code, expected) in [
        ("walk", None, "cleared"),
        ("step", None, "cleared"),
        ("gap", None, "cleared"),
        ("high", None, "unsolved"),
        ("high", Some("place(\"up\");"), "cleared"),
        ("move", Some("move(\"right\");"), "cleared"),
    ] {
        let result = solve(&scenario(name, code), 17, reach, 8).unwrap();
        println!(
            "{name} {code:?}: {} frames={} failures={}",
            result.status,
            result.frames,
            serde_json::to_string(&result.failures).unwrap()
        );
        assert_eq!(result.status, expected, "{name} {code:?}");
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
        assert_ne!(result.status, "error");
        assert!(result.attempts <= 8);
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
    assert!(cleared > 0);
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
    let reach = AgentReach { trials: vec![] };
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
