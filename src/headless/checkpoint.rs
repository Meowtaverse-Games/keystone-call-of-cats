//! Exact, replay-backed checkpoints: a correctness oracle, NOT a fast world clone.
//!
//! Bevy schedules contain opaque system-local state. Avian also owns persistent
//! contact/island caches. Copying only entities/resources would silently omit those
//! states. This prototype reconstructs the entire App from pinned terrain and the
//! input prefix, including the product script threads. Restore is O(frame), not O(1).
//! Checkpoints are process-local, immutable and reusable; there is no import API or
//! promise of compatibility across builds. Only frame-boundary input is supported.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyChange {
    pub key: InputKey,
    pub pressed: bool,
}

/// Owns exactly `size_of::<Self>() + bytes.len()` requested bytes, excluding the
/// allocator's bookkeeping/alignment. No shared world or VM is retained.
pub struct Snapshot {
    bytes: Box<[u8]>,
}
impl Snapshot {
    pub fn owned_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.bytes.len()
    }
}

/// Optional instrumentation installed only by checkpoint sessions. Sensory input
/// and yielded commands let replay validate script progress independently of the
/// physical trajectory. The global order also captures cross-stone scheduling.
#[derive(Resource, Default, Clone)]
pub(super) struct ScriptTrace(pub Arc<Mutex<Vec<ProgramCall>>>);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct ProgramCall {
    pub stone_index: usize,
    pub state: BTreeMap<String, String>,
    pub command: String,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    scenario: Scenario,
    ron_source: String,
    seed: u64,
    attempts: u64,
    overrides: BTreeMap<u64, Vec<KeyChange>>,
    prefix_digest: u64,
    program_trace: Vec<ProgramCall>,
    trajectory_hash: String,
    ended: bool,
}

/// All mutations go through `step`: exposing a mutable World would make the
/// reconstruction incomplete. Snapshot boundaries are after App::update/Last.
/// Frame-boundary sensing only; no entities or mutable world escape the session.
#[derive(Clone, Serialize)]
pub struct SensedState {
    pub frame: u64,
    pub goal_reached: bool,
    pub player_cell: [f32; 2],
    pub velocity: [f32; 2],
    pub grounded: bool,
    pub on_stone: Option<usize>,
    pub scene: TrajectoryPoint,
}

pub struct Session {
    app: App,
    sensing: bevy::ecs::system::SystemState<SpatialQuery<'static, 'static>>,
    run: SeedRun,
    ron_source: String,
    overrides: BTreeMap<u64, Vec<KeyChange>>,
    attempts: u64,
    ended: bool,
}
impl Session {
    pub fn new(scenario: Scenario, seed: u64) -> Result<Self> {
        validate(&scenario)?;
        let path = scenario_map_path(&scenario, &StageCatalog::load_from_assets())?;
        let source = fs::read_to_string(path)?;
        Self::from_source(scenario, seed, source)
    }

    fn from_source(scenario: Scenario, seed: u64, ron_source: String) -> Result<Self> {
        let config = ron::from_str::<ChunkGrammarConfig>(&ron_source)?;
        let map = chunk_grammar_map::generate_map_with_seed(config, seed);
        let run = SeedRun::new(scenario.clone(), seed, &map)?;
        let mut app = build_app_with_physics(
            map,
            scenario.viewport_scale,
            scenario.player_physics.resolve()?,
        );
        app.insert_resource(ScriptTrace::default());
        let sensing = bevy::ecs::system::SystemState::new(app.world_mut());
        Ok(Self {
            app,
            sensing,
            run,
            ron_source,
            overrides: BTreeMap::new(),
            attempts: 0,
            ended: false,
        })
    }

    pub fn frame(&self) -> u64 {
        self.run.report.frames
    }

    /// Advance one 60 Hz driver frame. Changes override scenario input for this
    /// frame, in order; held keys persist until released. Returns true at the end.
    /// A terminal session rejects further steps until restored.
    pub fn step(&mut self, changes: &[KeyChange]) -> Result<bool> {
        if self.ended {
            return Err(format!(
                "Cannot step a terminal session ({}: {:?}); restore a checkpoint first",
                self.run.report.termination, self.run.report.error
            )
            .into());
        }
        if !changes.is_empty() {
            self.overrides.insert(self.attempts, changes.to_vec());
        }
        self.attempts += 1;
        if !self.run.before_frame(self.app.world_mut()) {
            self.ended = true;
            return Ok(true);
        }
        for change in changes {
            if change.pressed {
                self.run.keys.press(change.key.code());
            } else {
                self.run.keys.release(change.key.code());
            }
        }
        *self.app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = self.run.keys.clone();
        self.app.update();
        // A failed observation is terminal; it must not leave a half-stepped run
        // looking usable to a caller.
        self.ended = true;
        self.ended = self.run.after_frame(self.app.world_mut())?;
        Ok(self.ended)
    }

    pub fn advance_to(&mut self, frame: u64) -> Result<()> {
        if frame < self.frame() {
            return Err("advance_to cannot rewind; use restore".into());
        }
        while self.frame() < frame {
            self.step(&[])?;
        }
        Ok(())
    }

    pub fn report(&self) -> Result<SeedReport> {
        self.run.clone().finish(self.app.world())
    }

    /// Read product transforms and the same foot shapecast used by player input.
    /// `&mut self` is needed only for Bevy query caches, never for simulation writes.
    pub fn observe(&mut self) -> Result<SensedState> {
        let world = self.app.world_mut();
        let (entity, transform, velocity, mask) = world
            .query_filtered::<(
                Entity,
                &GlobalTransform,
                &LinearVelocity,
                Option<&CollisionLayers>,
            ), With<Player>>()
            .iter(world)
            .next()
            .map(|(e, t, v, layers)| {
                (
                    e,
                    t.compute_transform(),
                    v.0,
                    layers.map_or(LayerMask::ALL, |l| l.filters),
                )
            })
            .ok_or("Player was not spawned")?;
        let physics = world.resource::<PlayerPhysicsParams>();
        let shape = Collider::rectangle(
            transform.scale.x * physics.ground_cast_width,
            transform.scale.y * physics.ground_cast_height,
        );
        let origin = transform.translation.truncate()
            - Vec2::Y * transform.scale.y * physics.ground_foot_offset;
        let mut config =
            ShapeCastConfig::from_max_distance(transform.scale.y * physics.ground_cast_distance);
        config.ignore_origin_penetration = false;
        let filter = SpatialQueryFilter::from_mask(mask).with_excluded_entities([entity]);
        let hit =
            self.sensing
                .get(world)?
                .cast_shape(&shape, origin, 0.0, Dir2::NEG_Y, &config, &filter);
        let on_stone = hit.and_then(|h| world.get::<StoneIndex>(h.entity).map(|i| i.0));
        let player_cell = cell_position(world, transform.translation.truncate()).to_array();
        let scene = snapshot(world, self.run.report.frames, &mut Digest::new())?;
        Ok(SensedState {
            frame: self.frame(),
            goal_reached: self.run.report.cleared,
            player_cell,
            velocity: velocity.to_array(),
            grounded: hit.is_some(),
            on_stone,
            scene,
        })
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let saved = Saved {
            scenario: self.run.scenario.clone(),
            ron_source: self.ron_source.clone(),
            seed: self.run.report.seed,
            attempts: self.attempts,
            overrides: self.overrides.clone(),
            prefix_digest: self.run.digest.0,
            program_trace: self
                .app
                .world()
                .resource::<ScriptTrace>()
                .0
                .lock()
                .expect("script trace lock")
                .clone(),
            trajectory_hash: self.report()?.trajectory_hash,
            ended: self.ended,
        };
        Ok(Snapshot {
            bytes: serde_json::to_vec(&saved)?.into_boxed_slice(),
        })
    }

    /// Transactional reconstruction: a mismatch leaves the current world intact.
    /// Rebuilds even private physics caches, RNG locals, message readers and VMs
    /// through their original schedules. Does not read the terrain file again.
    pub fn restore(&mut self, snapshot: &Snapshot) -> Result<()> {
        let saved: Saved = serde_json::from_slice(&snapshot.bytes)?;
        let mut restored = Self::from_source(saved.scenario, saved.seed, saved.ron_source)?;
        for attempt in 0..saved.attempts {
            restored.step(saved.overrides.get(&attempt).map_or(&[], Vec::as_slice))?;
        }
        let mismatch = if restored.run.digest.0 != saved.prefix_digest {
            Some("per-frame execution digest")
        } else if *restored
            .app
            .world()
            .resource::<ScriptTrace>()
            .0
            .lock()
            .expect("script trace lock")
            != saved.program_trace
        {
            Some("script sensory/command transcript")
        } else if restored.ended != saved.ended {
            Some("terminal flag")
        } else if restored.report()?.trajectory_hash != saved.trajectory_hash {
            Some("trajectory hash")
        } else {
            None
        };
        if let Some(field) = mismatch {
            return Err(format!(
                "Checkpoint replay diverged in {field}; current world was not replaced"
            )
            .into());
        }
        *self = restored;
        Ok(())
    }
}

#[derive(Serialize)]
struct Timing {
    mean_us: f64,
    max_us: f64,
}
impl Timing {
    fn from_seconds(samples: &[f64]) -> Self {
        Self {
            mean_us: samples.iter().sum::<f64>() / samples.len() as f64 * 1e6,
            max_us: samples.iter().copied().fold(0.0, f64::max) * 1e6,
        }
    }
}
#[derive(Serialize)]
struct Measurement {
    frame: u64,
    samples: usize,
    snapshot_owned_bytes: usize,
    snapshot: Timing,
    restore: Timing,
    replay_from_start: Timing,
}

/// Fixed protocol, no CI timing threshold. Includes initialization, VM compilation
/// and old-world destruction in restore; replay includes initialization and source
/// parsing too, but not filesystem IO. The warm-up is excluded for both methods.
pub(super) fn measure(scenario: Scenario) -> Result<serde_json::Value> {
    const SAMPLES: usize = 20;
    let mut rows = vec![];
    for frame in [10, 100, 1000] {
        let mut session = Session::new(scenario.clone(), scenario.seed_start)?;
        session.advance_to(frame)?;
        let checkpoint = session.snapshot()?;
        session.restore(&checkpoint)?;
        let mut snapshots = vec![];
        let mut restores = vec![];
        let mut replays = vec![];
        for index in 0..SAMPLES {
            let start = Instant::now();
            let snapshot = session.snapshot()?;
            snapshots.push(start.elapsed().as_secs_f64());
            std::hint::black_box(&snapshot);
            // Alternate ordering to reduce cache/temperature bias. Destruction of
            // the replay world is included, just as restore destroys its old world.
            let replay = || -> Result<f64> {
                let start = Instant::now();
                let mut fresh = Session::from_source(
                    scenario.clone(),
                    scenario.seed_start,
                    session.ron_source.clone(),
                )?;
                fresh.advance_to(frame)?;
                std::hint::black_box(&fresh);
                drop(fresh);
                Ok(start.elapsed().as_secs_f64())
            };
            if index % 2 == 0 {
                replays.push(replay()?);
            }
            let start = Instant::now();
            session.restore(&snapshot)?;
            restores.push(start.elapsed().as_secs_f64());
            if index % 2 != 0 {
                // End the immutable session borrow before restore above.
                let start = Instant::now();
                let mut fresh = Session::from_source(
                    scenario.clone(),
                    scenario.seed_start,
                    session.ron_source.clone(),
                )?;
                fresh.advance_to(frame)?;
                std::hint::black_box(&fresh);
                drop(fresh);
                replays.push(start.elapsed().as_secs_f64());
            }
        }
        rows.push(Measurement {
            frame,
            samples: SAMPLES,
            snapshot_owned_bytes: checkpoint.owned_bytes(),
            snapshot: Timing::from_seconds(&snapshots),
            restore: Timing::from_seconds(&restores),
            replay_from_start: Timing::from_seconds(&replays),
        });
    }
    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": "full-replay-correctness-oracle",
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "scenario": scenario,
        "memory_definition": "size_of::<Snapshot>() + owned boxed JSON bytes; excludes allocator overhead and transient reconstruction worlds",
        "crossover_frame": null,
        "crossover_note": "No algorithmic crossover: restore performs full replay plus validation. Small timing differences are noise, not evidence of acceleration.",
        "measurements": rows,
    }))
}

#[cfg(test)]
mod tests;
