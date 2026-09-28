//! Candidate evaluation for people and AI. These are grid-model checks, not Bevy acceptance.
use std::{collections::HashSet, fs, io::Write, path::PathBuf, time::Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use serde::Serialize;

use crate::{
    SimArgs,
    map::{StageConfig, generate, parse_stage},
    sim::{Metrics, World},
    source_path,
};

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    /// Require actual arrival at a goal after replay/automatic player walking.
    Goal,
    /// Require a player-only path, without claiming that it was played.
    Reachable,
    /// Require no player-only path with stationary stones/obstacles; not an unsolvability proof.
    Unreachable,
    /// Check generation and world construction only; not a playability verdict.
    Generated,
}

#[derive(Args)]
pub struct CheckArgs {
    #[command(flatten)]
    sim: SimArgs,
    /// Number of consecutive seeds, starting at --seed (1..=10000).
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=10_000))]
    seeds: u32,
    /// Manual simulator commands to replay for every seed, not Rhai/Keystone source.
    #[arg(long)]
    plan: Option<PathBuf>,
    /// After replay, find and actually execute a player-only goal route.
    #[arg(long)]
    walk_to_goal: bool,
    /// Reject a player-only shortcut before any stone operations.
    #[arg(long)]
    require_initially_unreachable: bool,
    /// Fail at the first blocked/no-effect player or stone action.
    #[arg(long)]
    reject_blocked: bool,
    #[arg(long, value_enum, default_value_t = Expect::Goal)]
    expect: Expect,
    /// New directory for report.json, source/plan snapshots and failed-seed evidence.
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Serialize)]
struct SeedResult {
    seed: u64,
    passed: bool,
    error: Option<String>,
    chunks: Vec<String>,
    initially_reachable: Option<bool>,
    goal_reached: bool,
    goal_reachable: bool,
    executed_plan: Vec<String>,
    initial_map: Option<String>,
    final_map: Option<String>,
    final_status: Option<String>,
    metrics: Metrics,
    elapsed_ms: u128,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    simulator_version: &'static str,
    model: &'static str,
    limitations: Vec<&'static str>,
    stage: usize,
    source: PathBuf,
    first_seed: u64,
    seeds: u32,
    expect: Expect,
    walk_to_goal: bool,
    require_initially_unreachable: bool,
    reject_blocked: bool,
    place_limit_override: Option<u32>,
    passed: usize,
    failed: usize,
    distinct_initial_layouts: usize,
    elapsed_ms: u128,
    results: Vec<SeedResult>,
}

pub fn run(args: &CheckArgs) -> Result<()> {
    let started = Instant::now();
    let last_seed = args
        .sim
        .stage
        .seed
        .checked_add(u64::from(args.seeds) - 1)
        .context("seed range overflows u64")?;
    let source = source_path(&args.sim.stage);
    let input = fs::read_to_string(&source)
        .with_context(|| format!("failed to read {}", source.display()))?;
    let plan = args
        .plan
        .as_ref()
        .map(fs::read_to_string)
        .transpose()
        .context("failed to read replay plan")?
        .unwrap_or_default();
    // Never overwrite previous evidence or the source candidate.
    if let Some(directory) = &args.output {
        fs::create_dir(directory).with_context(|| {
            format!(
                "cannot create new output directory {} (parent must exist; will not overwrite)",
                directory.display()
            )
        })?;
        fs::write(directory.join("source.ron"), &input)?;
        fs::write(directory.join("input.plan"), &plan)?;
    }
    // Parse errors also become structured failures for AI repair.
    let config = parse_stage(&input);
    let mut results = Vec::new();
    let mut distinct = HashSet::new();
    for seed in args.sim.stage.seed..=last_seed {
        let result = evaluate(
            config.as_ref().map_err(|e| format!("{e:#}")),
            seed,
            &plan,
            args,
        );
        if let Some(layout) = &result.initial_map {
            distinct.insert(layout.clone());
        }
        if !result.passed {
            eprintln!(
                "seed {seed}: FAIL {}",
                result.error.as_deref().unwrap_or("unknown")
            );
            if let Some(directory) = &args.output {
                fs::write(
                    directory.join(format!("seed-{seed}.json")),
                    serde_json::to_vec_pretty(&result)?,
                )?;
                fs::write(
                    directory.join(format!("seed-{seed}.plan")),
                    result.executed_plan.join("\n") + "\n",
                )?;
                if let Some(map) = &result.initial_map {
                    fs::write(directory.join(format!("seed-{seed}.txt")), map)?;
                }
            }
        }
        results.push(result);
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let report = Report {
        schema_version: 1,
        simulator_version: env!("CARGO_PKG_VERSION"),
        model: "discrete_grid",
        limitations: vec![
            "Not a Bevy physics, timing, human difficulty or fun verdict.",
            "Player-only paths hold stones and obstacles stationary; no stone-program search.",
            "Obstacle expiry, dynamic terrain randomization and fractional stone offsets are not simulated.",
            "CLI seeds are not product seeds. Use the captured RON with the same simulator revision.",
            "Plans are sequential simulator actions, not Rhai/Keystone programs or concurrent scheduling.",
        ],
        stage: args.sim.stage.stage,
        source,
        first_seed: args.sim.stage.seed,
        seeds: args.seeds,
        expect: args.expect,
        walk_to_goal: args.walk_to_goal,
        require_initially_unreachable: args.require_initially_unreachable,
        reject_blocked: args.reject_blocked,
        place_limit_override: args.sim.place_limit,
        passed,
        failed,
        distinct_initial_layouts: distinct.len(),
        elapsed_ms: started.elapsed().as_millis(),
        results,
    };
    if let Some(directory) = &args.output {
        fs::write(
            directory.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        println!("report={}", directory.join("report.json").display());
    }
    println!(
        "grid-check: passed={passed} failed={failed} layouts={} seeds={}..={} elapsed={}ms expect={:?}",
        report.distinct_initial_layouts,
        report.first_seed,
        last_seed,
        report.elapsed_ms,
        args.expect
    );
    std::io::stdout().flush()?;
    if failed > 0 {
        bail!("{failed} seed(s) did not meet the requested grid check");
    }
    Ok(())
}

fn evaluate(
    config: std::result::Result<&StageConfig, String>,
    seed: u64,
    plan: &str,
    args: &CheckArgs,
) -> SeedResult {
    let started = Instant::now();
    let mut result = SeedResult {
        seed,
        passed: false,
        error: None,
        chunks: Vec::new(),
        initially_reachable: None,
        goal_reached: false,
        goal_reachable: false,
        executed_plan: Vec::new(),
        initial_map: None,
        final_map: None,
        final_status: None,
        metrics: Metrics::default(),
        elapsed_ms: 0,
    };
    let evaluation = (|| -> Result<()> {
        let config = config.map_err(anyhow::Error::msg).context("parse")?;
        let map = generate(config, seed).context("generation")?;
        result.chunks = map.selected_chunks.clone();
        let mut world = World::from_map(&map, args.sim.place_limit).context("world")?;
        result.initial_map = Some(world.render(true));
        result.initially_reachable = Some(world.goal_is_reachable());
        let replay = (|| -> Result<()> {
            if args.require_initially_unreachable {
                result.executed_plan.push("assert unreachable".into());
                world
                    .execute("assert unreachable")
                    .context("initial shortcut check")?;
            }
            for (index, line) in plan.lines().enumerate() {
                let command = line.trim();
                if command.is_empty() || command.starts_with('#') {
                    continue;
                }
                result.executed_plan.push(command.to_owned());
                let blocked_before = world.metrics.blocked_actions;
                world
                    .execute(command)
                    .with_context(|| format!("plan line {}: {command}", index + 1))?;
                if args.reject_blocked && world.metrics.blocked_actions > blocked_before {
                    bail!(
                        "plan line {}: blocked/no-effect action: {command}",
                        index + 1
                    );
                }
            }
            if args.walk_to_goal {
                let route = world
                    .goal_route()
                    .context("no player-only route after plan")?;
                // Store concrete player actions, so successful checks contain a goal witness.
                for command in route {
                    result.executed_plan.push(command.clone());
                    world.execute(&command)?;
                }
            }
            let assertion = match args.expect {
                Expect::Goal => Some("assert goal"),
                Expect::Reachable => Some("assert reachable"),
                Expect::Unreachable => Some("assert unreachable"),
                Expect::Generated => None,
            };
            if let Some(assertion) = assertion {
                result.executed_plan.push(assertion.into());
                world.execute(assertion).context("final expectation")?;
            }
            Ok(())
        })();
        result.goal_reached = world.goal_reached();
        result.goal_reachable = world.goal_is_reachable();
        result.metrics = world.metrics.clone();
        result.final_status = Some(world.status());
        result.final_map = Some(world.render(true));
        replay
    })();
    result.passed = evaluation.is_ok();
    result.error = evaluation.err().map(|e| format!("{e:#}"));
    result.elapsed_ms = started.elapsed().as_millis();
    result
}
