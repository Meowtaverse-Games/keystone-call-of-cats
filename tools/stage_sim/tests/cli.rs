use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "stage-sim-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_stage_sim"))
}
fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}
fn report(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join("report.json")).unwrap()).unwrap()
}

#[test]
fn ai_candidate_has_multiple_layouts_and_replayable_goal_witnesses() {
    let scratch = Scratch::new();
    let output = scratch.path("report");
    let run = cli()
        .args(["check", "13", "--stage-file"])
        .arg(example("ai-candidate.ron"))
        .arg("--plan")
        .arg(example("ai-candidate.plan"))
        .args([
            "--seeds",
            "40",
            "--walk-to-goal",
            "--require-initially-unreachable",
            "--reject-blocked",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let report = report(&output);
    assert_eq!(report["passed"], 40);
    assert_eq!(report["failed"], 0);
    assert!(report["distinct_initial_layouts"].as_u64().unwrap() > 1);
    for result in report["results"].as_array().unwrap() {
        assert_eq!(result["initially_reachable"], false);
        assert_eq!(result["goal_reached"], true);
        assert_eq!(result["metrics"]["blocked_actions"], 0);
    }
    let witness = scratch.path("witness.plan");
    let actions = report["results"][7]["executed_plan"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&witness, actions).unwrap();
    let replay = cli()
        .args(["simulate", "13", "--seed", "7", "--stage-file"])
        .arg(output.join("source.ron"))
        .arg("--plan")
        .arg(witness)
        .arg("--require-goal")
        .output()
        .unwrap();
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
}

#[test]
fn reachable_is_not_reported_as_actual_arrival() {
    let scratch = Scratch::new();
    for (expect, success) in [("reachable", true), ("goal", false)] {
        let output = scratch.path(expect);
        let run = cli()
            .args(["check", "13", "--stage-file"])
            .arg(example("ai-candidate.ron"))
            .arg("--plan")
            .arg(example("ai-candidate.plan"))
            .args(["--expect", expect, "--output"])
            .arg(&output)
            .output()
            .unwrap();
        assert_eq!(run.status.success(), success);
        let result = &report(&output)["results"][0];
        assert_eq!(result["goal_reachable"], true);
        assert_eq!(result["goal_reached"], false);
    }
}

#[test]
fn all_failing_seeds_are_saved_with_immutable_source_and_reproducible_failure() {
    let scratch = Scratch::new();
    let output = scratch.path("failed");
    let run = cli()
        .args(["check", "13", "--stage-file"])
        .arg(example("ai-candidate.ron"))
        .arg("--plan")
        .arg(example("ai-candidate.plan"))
        .args([
            "--place-limit",
            "0",
            "--seeds",
            "3",
            "--walk-to-goal",
            "--reject-blocked",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert_eq!(report(&output)["failed"], 3);
    assert_eq!(
        fs::read(output.join("source.ron")).unwrap(),
        fs::read(example("ai-candidate.ron")).unwrap()
    );
    for seed in 0..3 {
        assert!(output.join(format!("seed-{seed}.json")).exists());
        assert!(output.join(format!("seed-{seed}.txt")).exists());
    }
    let replay = cli()
        .args(["check", "13", "--stage-file"])
        .arg(output.join("source.ron"))
        .arg("--plan")
        .arg(output.join("input.plan"))
        .args([
            "--seed",
            "2",
            "--place-limit",
            "0",
            "--reject-blocked",
            "--walk-to-goal",
        ])
        .output()
        .unwrap();
    assert!(!replay.status.success());
    assert!(String::from_utf8_lossy(&replay.stderr).contains("blocked/no-effect"));
    let original = fs::read(output.join("report.json")).unwrap();
    let second = cli()
        .args(["check", "13", "--stage-file"])
        .arg(example("ai-candidate.ron"))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert_eq!(fs::read(output.join("report.json")).unwrap(), original);
}

#[test]
fn invalid_ai_input_is_a_structured_failure_not_a_success() {
    let scratch = Scratch::new();
    for (name, source) in [
        ("syntax", "not valid RON".to_owned()),
        (
            "unknown-budget",
            fs::read_to_string(example("ai-candidate.ron"))
                .unwrap()
                .replace("place_limit", "place_limti"),
        ),
        (
            "dimensions",
            fs::read_to_string(example("ai-candidate.ron"))
                .unwrap()
                .replace("(28, 18)", "(100, 100)"),
        ),
    ] {
        let candidate = scratch.path(&format!("{name}.ron"));
        fs::write(&candidate, source).unwrap();
        let output = scratch.path(name);
        let run = cli()
            .args(["check", "1", "--stage-file"])
            .arg(candidate)
            .args(["--expect", "generated", "--output"])
            .arg(&output)
            .output()
            .unwrap();
        assert!(!run.status.success());
        let report = report(&output);
        assert_eq!(report["failed"], 1);
        assert!(report["results"][0]["error"].as_str().is_some());
        assert_eq!(report["results"][0]["goal_reached"], false);
    }
}

#[test]
fn recorded_interactive_play_replays_without_overwriting_files() {
    let scratch = Scratch::new();
    let recording = scratch.path("play.plan");
    let mut child = cli()
        .args(["play", "13", "--stage-file"])
        .arg(example("ai-candidate.ron"))
        .arg("--record")
        .arg(&recording)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"s 0 place up\nwalk-goal\nassert goal\nquit\n")
        .unwrap();
    let run = child.wait_with_output().unwrap();
    assert!(run.status.success());
    let contents = fs::read_to_string(&recording).unwrap();
    assert!(contents.contains("s 0 place up"));
    assert!(contents.contains("p "));
    assert!(
        !contents.contains("walk-goal"),
        "record concrete player actions"
    );
    let run = cli()
        .args(["simulate", "13", "--stage-file"])
        .arg(example("ai-candidate.ron"))
        .arg("--plan")
        .arg(&recording)
        .arg("--require-goal")
        .output()
        .unwrap();
    assert!(run.status.success());
    let second = cli()
        .args(["play", "13", "--stage-file"])
        .arg(example("ai-candidate.ron"))
        .arg("--record")
        .arg(&recording)
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert_eq!(fs::read_to_string(&recording).unwrap(), contents);
}

#[test]
fn seed_overflow_and_empty_batches_are_rejected() {
    for args in [
        vec!["--seed", "18446744073709551615", "--seeds", "2"],
        vec!["--seeds", "0"],
    ] {
        let run = cli()
            .args(["check", "13", "--stage-file"])
            .arg(example("ai-candidate.ron"))
            .args(args)
            .output()
            .unwrap();
        assert!(!run.status.success());
    }
}
