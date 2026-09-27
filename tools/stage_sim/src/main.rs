mod map;
mod sim;

use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use keystone_lang::{Direction, Event, ExternalApi};
use map::{GeneratedMap, TileKind, generate, parse_stage};
use sim::{HELP, World};

#[derive(Parser)]
#[command(
    name = "stage-sim",
    about = "Bevy-free ASCII stage renderer and grid design simulator"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render one generated stage as ASCII.
    Render(StageArgs),
    /// Print structural metrics and approximate player reachability.
    Analyze(StageArgs),
    /// Manipulate the player and stones in an interactive prompt.
    Play(SimArgs),
    /// Replay a text plan and optionally print every frame.
    Simulate {
        #[command(flatten)]
        sim: SimArgs,
        /// Text file containing one simulator command per line.
        #[arg(long)]
        plan: PathBuf,
        /// Print the map after every state-changing command.
        #[arg(long)]
        frames: bool,
    },
    /// Execute real Keystone-language programs against the grid model.
    Run {
        #[command(flatten)]
        sim: SimArgs,
        /// Keystone source file for a stone; repeat in stone index order.
        #[arg(long = "stone-script", required = true)]
        stone_scripts: Vec<PathBuf>,
        /// Optional player actions, one simulator command per program round.
        #[arg(long)]
        player_plan: Option<PathBuf>,
        /// Safety limit for round-robin program execution.
        #[arg(long, default_value_t = 1_000)]
        max_rounds: usize,
        /// Print the ASCII map after rounds that change the world.
        #[arg(long)]
        frames: bool,
    },
}

#[derive(Args, Clone)]
struct StageArgs {
    /// Stage number, resolved as assets/stages/stage-N.ron.
    stage: usize,
    /// Deterministic chunk-layout seed.
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Root containing stage-N.ron files.
    #[arg(long, default_value = "assets/stages")]
    stages_dir: PathBuf,
    /// Show x/y coordinates around the ASCII map.
    #[arg(long)]
    coordinates: bool,
}

#[derive(Args, Clone)]
struct SimArgs {
    #[command(flatten)]
    stage: StageArgs,
    /// Number of blocks available to the proposed place command.
    #[arg(long, default_value_t = 0)]
    place_limit: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render(args) => {
            let generated = load_map(&args)?;
            print_header(args.stage, args.seed, &generated);
            let world = World::from_map(&generated, None)?;
            print!("{}", world.render(args.coordinates));
            print_legend();
        }
        Command::Analyze(args) => {
            let generated = load_map(&args)?;
            analyze(args.stage, args.seed, &generated)?;
        }
        Command::Play(args) => play(&args)?,
        Command::Simulate { sim, plan, frames } => simulate(&sim, &plan, frames)?,
        Command::Run {
            sim,
            stone_scripts,
            player_plan,
            max_rounds,
            frames,
        } => run_programs(
            &sim,
            &stone_scripts,
            player_plan.as_deref(),
            max_rounds,
            frames,
        )?,
    }
    Ok(())
}

fn load_map(args: &StageArgs) -> Result<GeneratedMap> {
    let path = stage_path(&args.stages_dir, args.stage);
    let input =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config =
        parse_stage(&input).with_context(|| format!("failed to parse {}", path.display()))?;
    generate(&config, args.seed).with_context(|| format!("failed to generate stage {}", args.stage))
}

fn stage_path(root: &Path, stage: usize) -> PathBuf {
    root.join(format!("stage-{stage}.ron"))
}

fn print_header(stage: usize, seed: u64, map: &GeneratedMap) {
    println!("stage={stage} seed={seed}");
    println!(
        "design-size={:?} canvas-size={:?} margin={:?}",
        map.design_size, map.canvas_size, map.boundary_margin
    );
    println!("chunks={}", map.selected_chunks.join(" -> "));
}

fn print_legend() {
    println!(
        "legend: @ player, S/0-9 stones, * solid, # boundary, O obstacle, ? dynamic, + placed, G goal"
    );
}

fn analyze(stage: usize, seed: u64, map: &GeneratedMap) -> Result<()> {
    print_header(stage, seed, map);
    let world = World::from_map(map, None)?;
    println!(
        "stone-type={:?} stones={} dig-limit={} dynamic={}-{}",
        map.stone_type,
        map.positions(TileKind::Stone).len(),
        map.dig_limit
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unlimited".to_owned()),
        map.dynamic_min,
        map.dynamic_max,
    );
    println!(
        "tiles: solid={} obstacle={} dynamic={} goals={}",
        map.positions(TileKind::Solid).len(),
        map.positions(TileKind::Obstacle).len(),
        map.positions(TileKind::DynamicSolid).len(),
        map.positions(TileKind::Goal).len(),
    );
    if !map.stone_adjustments.is_empty() {
        println!(
            "note: fractional stone adjustments {:?} are displayed on their source grid cells",
            map.stone_adjustments
        );
    }
    let reachable = world.reachable_positions();
    println!(
        "approx-player-reachability: cells={} goal={} (static stones, jump-height<=2, jump-width<=2)",
        reachable.len(),
        if world.goal_is_reachable() {
            "reachable"
        } else {
            "requires-stage-actions"
        }
    );
    Ok(())
}

fn play(args: &SimArgs) -> Result<()> {
    let map = load_map(&args.stage)?;
    let mut world = World::from_map(&map, Some(args.place_limit))?;
    print_header(args.stage.stage, args.stage.seed, &map);
    print!("{}", world.render(true));
    print_legend();
    println!("{HELP}");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        match line.trim() {
            "quit" | "exit" => break,
            "help" => println!("{HELP}"),
            command => match world.execute(command) {
                Ok(output) if !output.is_empty() => println!("{output}"),
                Ok(_) => {}
                Err(error) => eprintln!("error: {error:#}"),
            },
        }
    }
    println!("final: {}", world.status());
    Ok(())
}

fn simulate(args: &SimArgs, plan_path: &Path, frames: bool) -> Result<()> {
    let map = load_map(&args.stage)?;
    let mut world = World::from_map(&map, Some(args.place_limit))?;
    let plan = fs::read_to_string(plan_path)
        .with_context(|| format!("failed to read plan {}", plan_path.display()))?;

    print_header(args.stage.stage, args.stage.seed, &map);
    for (index, line) in plan.lines().enumerate() {
        let command = line.trim();
        if command.is_empty() || command.starts_with('#') {
            continue;
        }
        if matches!(command, "quit" | "exit") {
            break;
        }
        if command == "help" {
            println!("{HELP}");
            continue;
        }
        let output = world
            .execute(command)
            .with_context(|| format!("{}:{}: {command}", plan_path.display(), index + 1))?;
        println!("{:>3}: {:<36} {output}", index + 1, command);
        if frames && is_state_command(command) {
            print!("{}", world.render(args.stage.coordinates));
        }
    }
    println!("final: {}", world.status());
    if !world.goal_reached() && !world.goal_is_reachable() {
        bail!("plan ended with the goal unreachable in the abstract simulator")
    }
    Ok(())
}

fn is_state_command(command: &str) -> bool {
    command.starts_with("p ")
        || command.starts_with("player ")
        || command.starts_with("s ")
        || command.starts_with("stone ")
        || command == "reset"
}

#[derive(Clone)]
struct SimulatorApi {
    world: Arc<Mutex<World>>,
    stone_index: usize,
    signals: Arc<Mutex<HashSet<String>>>,
}

impl ExternalApi for SimulatorApi {
    fn is_touched(&self) -> bool {
        self.world
            .lock()
            .map(|world| world.program_is_touched(self.stone_index))
            .unwrap_or(false)
    }

    fn is_empty(&self, direction: Direction) -> bool {
        let Some(delta) = direction_delta(direction) else {
            return false;
        };
        self.world
            .lock()
            .map(|world| world.program_is_empty(self.stone_index, delta))
            .unwrap_or(false)
    }

    fn send_signal(&self, channel: &str) {
        if let Ok(mut signals) = self.signals.lock() {
            signals.insert(channel.to_owned());
        }
    }

    fn receive_signal(&self, channel: &str) -> bool {
        self.signals
            .lock()
            .map(|mut signals| signals.remove(channel))
            .unwrap_or(false)
    }
}

fn run_programs(
    args: &SimArgs,
    script_paths: &[PathBuf],
    player_plan_path: Option<&Path>,
    max_rounds: usize,
    frames: bool,
) -> Result<()> {
    let map = load_map(&args.stage)?;
    let world = Arc::new(Mutex::new(World::from_map(&map, Some(args.place_limit))?));
    let signals = Arc::new(Mutex::new(HashSet::new()));
    let mut programs = Vec::new();

    for (stone_index, path) in script_paths.iter().enumerate() {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read script {}", path.display()))?;
        let api = SimulatorApi {
            world: Arc::clone(&world),
            stone_index,
            signals: Arc::clone(&signals),
        };
        let iterator = keystone_lang::eval(&source, Arc::new(api))
            .map_err(|error| anyhow::anyhow!("{}: {error:?}", path.display()))?;
        programs.push((path.clone(), iterator, false, 0_u32));
    }

    let stone_count = world
        .lock()
        .map_err(|_| anyhow::anyhow!("world lock poisoned"))?
        .status();
    if script_paths.is_empty() {
        bail!("at least one --stone-script is required");
    }
    print_header(args.stage.stage, args.stage.seed, &map);
    println!("programs={} initial={stone_count}", script_paths.len());

    let player_actions = if let Some(path) = player_plan_path {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read player plan {}", path.display()))?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    for round in 0..max_rounds {
        let mut changed = false;
        if let Some(action) = player_actions.get(round)
            && action != "wait"
        {
            let output = world
                .lock()
                .map_err(|_| anyhow::anyhow!("world lock poisoned"))?
                .execute(action)
                .with_context(|| format!("player round {}: {action}", round + 1))?;
            println!("round {:>3} player: {output}", round + 1);
            changed |= is_state_command(action);
        }

        let mut any_active = false;
        for (stone_index, (path, iterator, finished, sleep_remaining)) in
            programs.iter_mut().enumerate()
        {
            if *finished {
                continue;
            }
            any_active = true;
            if *sleep_remaining > 0 {
                *sleep_remaining -= 1;
                continue;
            }
            match iterator.next() {
                Some(Ok(event)) => {
                    let (description, event_changed, event_sleep) =
                        apply_event(&world, stone_index, event).with_context(|| {
                            format!("{} stone {stone_index} round {}", path.display(), round + 1)
                        })?;
                    *sleep_remaining = event_sleep;
                    if let Some(description) = description {
                        println!("round {:>3} stone {stone_index}: {description}", round + 1);
                    }
                    changed |= event_changed;
                }
                Some(Err(error)) => {
                    bail!("{} stone {stone_index}: {error:?}", path.display())
                }
                None => *finished = true,
            }
        }

        if frames && changed {
            print!(
                "{}",
                world
                    .lock()
                    .map_err(|_| anyhow::anyhow!("world lock poisoned"))?
                    .render(args.stage.coordinates)
            );
        }
        if world
            .lock()
            .map_err(|_| anyhow::anyhow!("world lock poisoned"))?
            .goal_reached()
        {
            println!("goal reached at round {}", round + 1);
            break;
        }
        if !any_active && round >= player_actions.len() {
            break;
        }
        if round + 1 == max_rounds {
            bail!("program execution exceeded --max-rounds={max_rounds}")
        }
    }

    let world = world
        .lock()
        .map_err(|_| anyhow::anyhow!("world lock poisoned"))?;
    println!("final: {}", world.status());
    println!(
        "goal-reachable={} (abstract grid model)",
        world.goal_is_reachable()
    );
    if player_plan_path.is_some() && !world.goal_reached() {
        bail!("player plan ended before reaching the goal")
    }
    Ok(())
}

fn apply_event(
    world: &Arc<Mutex<World>>,
    stone_index: usize,
    event: Event,
) -> Result<(Option<String>, bool, u32)> {
    let direction_name = |direction| match direction {
        Direction::Left => Some("left"),
        Direction::Right => Some("right"),
        Direction::Up => Some("up"),
        Direction::Down => Some("down"),
        Direction::Forward | Direction::Back => None,
    };
    match event {
        Event::Move(direction) => {
            let Some(direction) = direction_name(direction) else {
                bail!("forward/back movement is not supported by the stage grid")
            };
            let output = world
                .lock()
                .map_err(|_| anyhow::anyhow!("world lock poisoned"))?
                .execute(&format!("s {stone_index} move {direction}"))?;
            Ok((Some(output), true, 0))
        }
        Event::Dig(direction) => {
            let Some(direction) = direction_name(direction) else {
                bail!("forward/back digging is not supported by the stage grid")
            };
            let output = world
                .lock()
                .map_err(|_| anyhow::anyhow!("world lock poisoned"))?
                .execute(&format!("s {stone_index} dig {direction}"))?;
            Ok((Some(output), true, 0))
        }
        Event::Sleep(seconds) => {
            let rounds = seconds.max(0.0).ceil() as u32;
            Ok((
                Some(format!("sleep {seconds} ({rounds} design rounds)")),
                false,
                rounds.saturating_sub(1),
            ))
        }
        Event::Print(value) => Ok((Some(format!("print {value}")), false, 0)),
        Event::Send(channel) => Ok((Some(format!("send {channel}")), false, 0)),
        Event::Receive(channel) => Ok((Some(format!("receive {channel}")), false, 0)),
        Event::Wait => Ok((Some("wait".to_owned()), false, 0)),
        Event::Turn(_) | Event::Let | Event::Tick => Ok((None, false, 0)),
    }
}

fn direction_delta(direction: Direction) -> Option<(i32, i32)> {
    match direction {
        Direction::Left => Some((-1, 0)),
        Direction::Right => Some((1, 0)),
        Direction::Up => Some((0, 1)),
        Direction::Down => Some((0, -1)),
        Direction::Forward | Direction::Back => None,
    }
}
