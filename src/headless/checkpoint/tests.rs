use super::*;
use crate::scenes::stage::components::{DugTile, PlayerGoalDescent};

fn scenario(path: &str) -> Scenario {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
fn complete(session: &mut Session) -> SeedReport {
    while !session.ended {
        session.step(&[]).unwrap();
    }
    session.report().unwrap()
}

// Extra state beyond trajectory_hash: contact manifolds (including warm-start
// impulses), sleeping pairs/timers, physics islands, body positions/velocities,
// collider presence/disabled state, and the fixed-clock accumulator. Private
// scratch caches and system locals are reconstructed, not individually inspected.
fn internal_state(session: &mut Session) -> String {
    use avian2d::{
        collision::contact_types::ContactGraph, dynamics::solver::islands::PhysicsIslands,
    };
    let world = session.app.world_mut();
    let fixed = world.resource::<Time<Fixed>>();
    let time = (
        fixed.elapsed(),
        fixed.delta(),
        fixed.overstep(),
        fixed.timestep(),
        world.resource::<Time>().elapsed(),
        world.resource::<Time<Physics>>().elapsed(),
    );
    let contacts = world.resource::<ContactGraph>();
    let mut active: Vec<_> = contacts.iter_active().map(|p| format!("{p:?}")).collect();
    let mut sleeping: Vec<_> = contacts.iter_sleeping().map(|p| format!("{p:?}")).collect();
    active.sort();
    sleeping.sort();
    let islands = format!("{:?}", world.resource::<PhysicsIslands>());
    let constraints = format!(
        "{:?}",
        world.resource::<avian2d::dynamics::solver::constraint_graph::ConstraintGraph>()
    );
    let mut bodies: Vec<_> = world
        .query::<(
            Entity,
            &Position,
            &Rotation,
            Option<&LinearVelocity>,
            Option<&AngularVelocity>,
            Option<&SleepTimer>,
            Has<Sleeping>,
            Has<Collider>,
            Has<ColliderDisabled>,
        )>()
        .iter(world)
        .map(|row| format!("{row:?}"))
        .collect();
    bodies.sort();
    format!("{time:?}\n{active:?}\n{sleeping:?}\n{islands}\n{constraints}\n{bodies:?}")
}

fn exercise(s: Scenario, frames: &[u64]) {
    // Unmodified straight-through driver is the oracle, not a second checkpoint run.
    let expected = verify_seed(&s, s.seed_start).unwrap();
    let mut run = Session::new(s.clone(), s.seed_start).unwrap();
    let mut checkpoints = vec![];
    for &frame in frames {
        run.advance_to(frame).unwrap();
        checkpoints.push((frame, run.snapshot().unwrap(), internal_state(&mut run)));
    }
    let a = complete(&mut run);
    assert_eq!(a.trajectory_hash, expected.trajectory_hash);
    assert_eq!(a.execution_hash, expected.execution_hash);
    for (frame, checkpoint, internal) in checkpoints.into_iter().rev() {
        run.restore(&checkpoint).unwrap();
        assert_eq!(run.frame(), frame);
        assert_eq!(internal_state(&mut run), internal, "state at frame {frame}");
        let b = complete(&mut run);
        assert_eq!(
            a.trajectory_hash, b.trajectory_hash,
            "straight restore {frame}"
        );
        assert_eq!(
            serde_json::to_value(&a.trajectory).unwrap(),
            serde_json::to_value(&b.trajectory).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&a.commands).unwrap(),
            serde_json::to_value(&b.commands).unwrap()
        );
        run.restore(&checkpoint).unwrap();
        if !run.ended {
            run.step(&[
                KeyChange {
                    key: InputKey::ArrowRight,
                    pressed: false,
                },
                KeyChange {
                    key: InputKey::ArrowLeft,
                    pressed: true,
                },
                KeyChange {
                    key: InputKey::Space,
                    pressed: true,
                },
            ])
            .unwrap();
            for _ in 0..20 {
                if run.ended {
                    break;
                }
                run.step(&[]).unwrap();
            }
        }
        run.restore(&checkpoint).unwrap();
        assert_eq!(internal_state(&mut run), internal);
        assert_eq!(
            complete(&mut run).trajectory_hash,
            a.trajectory_hash,
            "branch restore {frame}"
        );
    }
}

#[test]
fn idle_walk_jump_landing_wall_and_airborne_resize() {
    exercise(
        scenario("examples/headless/motion-walk-stop.json"),
        &[0, 45, 100, 310],
    );
    let wall = scenario("examples/headless/motion-wall-hit.json");
    let mut wall_probe = Session::new(wall.clone(), 17).unwrap();
    wall_probe.advance_to(219).unwrap();
    let x_before = player_position(&mut wall_probe.app).unwrap().x;
    wall_probe.step(&[]).unwrap();
    let x_after = player_position(&mut wall_probe.app).unwrap().x;
    assert!(
        (x_before - x_after).abs() < 0.01,
        "player must be blocked by the wall"
    );
    exercise(wall, &[220]);
    let mut jump = scenario("examples/headless/motion-standing-jump.json");
    // Assert that the saved frames really cover both halves of the jump and the
    // first grounded frame. Do not silently label arbitrary frame numbers.
    let mut probe = Session::new(jump.clone(), 17).unwrap();
    let mut rising = None;
    let mut falling = None;
    let mut landed = None;
    while probe.frame() < 250 {
        probe.step(&[]).unwrap();
        let world = probe.app.world_mut();
        let velocity = world
            .query_filtered::<&LinearVelocity, With<Player>>()
            .single(world)
            .unwrap();
        if probe.run.report.frames > 90 && velocity.y > 1.0 {
            rising.get_or_insert(probe.run.report.frames);
        }
        if rising.is_some() && velocity.y < -1.0 {
            falling.get_or_insert(probe.run.report.frames);
        }
        if falling.is_some() && velocity.y.abs() < 1.0 {
            landed.get_or_insert(probe.run.report.frames);
        }
    }
    let frames = [rising.unwrap(), falling.unwrap(), landed.unwrap()];
    exercise(jump.clone(), &frames);
    jump.scale_changes = vec![ScaleChange {
        frame: frames[0] + 5,
        scale: 1.5,
    }];
    exercise(jump, &[frames[0] + 6, frames[1] + 10]);
}

#[test]
fn obstacle_vanish_before_after_and_dynamic_terrain() {
    let mut s = scenario("examples/headless/snapshot-terrain.json");
    let mut probe = Session::new(s.clone(), 17).unwrap();
    let mut vanished = None;
    let mut previous = 0;
    while !probe.ended {
        probe.step(&[]).unwrap();
        let world = probe.app.world_mut();
        let count = world
            .query_filtered::<Entity, (With<systems::AnimatedObstacle>, With<Collider>)>()
            .iter(world)
            .count();
        if previous > count {
            vanished.get_or_insert(probe.run.report.frames);
        }
        previous = count;
    }
    let vanished = vanished.expect("fixture must remove an obstacle collider");
    exercise(s.clone(), &[1, vanished - 1, vanished, vanished + 15]);
    // Dynamic terrain changes on StoneTick, not with wall-clock time. Without
    // programs it changes on the initial reset; use moving stones to test later
    // RNG draws as well. Both use the same pinned synthetic RON.
    for (language, code) in [
        (Language::Rhai, "loop { move(\"right\"); move(\"left\"); }"),
        (Language::Keystone, "loop 20\nmove right\nmove left\nend"),
    ] {
        s.max_frames = 240;
        s.programs = vec![StoneProgram {
            stone_index: 0,
            language,
            code: code.into(),
        }];
        let mut probe = Session::new(s.clone(), 17).unwrap();
        let mut masks = std::collections::HashSet::new();
        for _ in 0..200 {
            probe.step(&[]).unwrap();
            let world = probe.app.world_mut();
            let mut ids: Vec<_> = world
                .query_filtered::<Entity, With<ColliderDisabled>>()
                .iter(world)
                .map(Entity::to_bits)
                .collect();
            ids.sort();
            masks.insert(ids);
        }
        assert!(masks.len() > 1, "dynamic terrain must actually change");
        exercise(s.clone(), &[1, 65, 130]);
    }
}

#[test]
fn rhai_and_keystone_dig_and_place() {
    for (fixture, command) in [("dig", "dig"), ("snapshot-place", "place")] {
        let mut s = scenario(&format!("examples/headless/{fixture}.json"));
        for language in [Language::Rhai, Language::Keystone] {
            // Yield before and after the edit so checkpoints cover a suspended VM
            // as well as the resulting modified terrain.
            s.programs[0].language = language;
            s.programs[0].code = match (language, command) {
                (Language::Rhai, "dig") => "sleep(0.2); if !is_empty(\"down\") { dig(\"down\"); } sleep(0.2); move(\"right\");".into(),
                (Language::Keystone, "dig") => "sleep 0.2\nif not is_empty(down)\ndig down\nend\nsleep 0.2\nmove right".into(),
                (Language::Rhai, _) => "place(\"up\"); move(\"right\"); move(\"left\");".into(),
                (Language::Keystone, _) => "place up\nmove right\nmove left".into(),
            };
            let mut probe = Session::new(s.clone(), 17).unwrap();
            probe.advance_to(120).unwrap();
            let world = probe.app.world_mut();
            let count = if command == "dig" {
                world
                    .query_filtered::<Entity, With<DugTile>>()
                    .iter(world)
                    .count()
            } else {
                world
                    .query_filtered::<Entity, With<systems::PlacedTile>>()
                    .iter(world)
                    .count()
            };
            assert!(
                count > 0,
                "{language:?} {command} must change terrain: {}",
                serde_json::to_string(&probe.report().unwrap().commands).unwrap()
            );
            assert!(
                probe
                    .report()
                    .unwrap()
                    .commands
                    .iter()
                    .any(|c| c.completed_frame.is_some())
            );
            exercise(s.clone(), &[5, 30, 120]);
        }
    }
}

#[test]
fn goal_descent() {
    let s = scenario("examples/headless/motion-goal-descent.json");
    let baseline = verify_seed(&s, 17).unwrap();
    let frame = baseline.goal_frame.unwrap() + 10;
    let mut probe = Session::new(s.clone(), 17).unwrap();
    probe.advance_to(frame).unwrap();
    let world = probe.app.world_mut();
    assert_eq!(world.query::<&PlayerGoalDescent>().iter(world).count(), 1);
    exercise(s, &[frame]);
}

#[test]
fn checkpoints_on_branches_survive_other_snapshots_and_source_removal() {
    let mut s = scenario("examples/headless/motion-standing-jump.json");
    // Pin source bytes without creating any filesystem artifact in a test.
    let source = fs::read_to_string(s.ron_path.as_ref().unwrap()).unwrap();
    s.ron_path = Some("target/does-not-exist-checkpoint-source.ron".into());
    let mut run = Session::from_source(s, 17, source).unwrap();
    run.advance_to(100).unwrap();
    let original = run.snapshot().unwrap();
    run.step(&[KeyChange {
        key: InputKey::ArrowRight,
        pressed: true,
    }])
    .unwrap();
    run.advance_to(120).unwrap();
    let branch = run.snapshot().unwrap();
    let expected_branch = complete(&mut run).trajectory_hash;
    assert!(run.step(&[]).is_err());
    run.restore(&original).unwrap();
    let original_hash = complete(&mut run).trajectory_hash;
    assert_ne!(
        expected_branch, original_hash,
        "branch must change the trajectory"
    );
    run.restore(&branch).unwrap();
    assert_eq!(complete(&mut run).trajectory_hash, expected_branch);
    run.restore(&original).unwrap();
    assert_eq!(complete(&mut run).trajectory_hash, original_hash);
}

#[test]
fn failed_restore_is_transactional_and_terminal_checkpoints_work() {
    let mut run = Session::new(scenario("examples/headless/motion-walk-stop.json"), 17).unwrap();
    run.advance_to(50).unwrap();
    let checkpoint = run.snapshot().unwrap();
    let mut saved: Saved = serde_json::from_slice(&checkpoint.bytes).unwrap();
    saved.prefix_digest ^= 1;
    let invalid = Snapshot {
        bytes: serde_json::to_vec(&saved).unwrap().into_boxed_slice(),
    };
    let before = internal_state(&mut run);
    assert!(run.restore(&invalid).is_err());
    assert_eq!(internal_state(&mut run), before);
    let terminal_hash = complete(&mut run).trajectory_hash;
    let terminal = run.snapshot().unwrap();
    run.restore(&checkpoint).unwrap();
    run.restore(&terminal).unwrap();
    assert!(run.ended);
    assert_eq!(run.report().unwrap().trajectory_hash, terminal_hash);
    assert!(run.step(&[]).is_err());
}

#[test]
fn compile_and_runtime_errors_are_reproducible_terminal_boundaries() {
    for code in ["place(\"up\");", "loop {}"] {
        let mut s = scenario("examples/headless/dig.json");
        s.programs[0].code = code.into();
        let mut run = Session::new(s.clone(), 17).unwrap();
        let baseline = verify_seed(&s, 17).unwrap();
        let initial = run.snapshot().unwrap();
        assert!(run.step(&[]).unwrap());
        let terminal = run.snapshot().unwrap();
        assert_eq!(
            run.report().unwrap().trajectory_hash,
            baseline.trajectory_hash
        );
        run.restore(&initial).unwrap();
        run.restore(&terminal).unwrap();
        assert!(run.ended);
        assert_eq!(
            run.report().unwrap().trajectory_hash,
            baseline.trajectory_hash
        );
    }
}
