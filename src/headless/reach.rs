//! Empirical landing trials, using the same entity construction and physics as play.
use super::*;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct ReachReport {
    schema_version: u32,
    viewport_px: [f32; 2],
    tile_px: f32,
    hypothesis_equal_cell_reach: bool,
    protocol: &'static str,
    measurements: Vec<Measurement>,
}
#[derive(Serialize)]
struct Measurement {
    map_size: [isize; 2],
    direction: &'static str,
    approach: &'static str,
    max_step_cells: isize,
    max_step_px: f32,
    max_gap_cells: isize,
    max_gap_px: f32,
    trials: Vec<Trial>,
}
#[derive(Serialize)]
struct Trial {
    requested_takeoff_cell: f32,
    actual_takeoff_cell: Option<f32>,
    kind: &'static str,
    cells: isize,
    px: f32,
    landed: bool,
    jumped: bool,
    peak_rise_px: f32,
    horizontal_travel_px: f32,
}

fn trial(
    size: (isize, isize),
    left: bool,
    runup: bool,
    step: bool,
    cells: isize,
    takeoff_cell: f32,
) -> Result<Trial> {
    // Bottom-up interior cells. A fixed source edge and mirrored destination make
    // the two directions comparable without teleporting or changing any physics.
    let reflect = |x: isize| if left { size.0 - 1 - x } else { x };
    let landing_x = 7 + if step { 0 } else { cells };
    let height = if step { cells } else { 0 };
    let mut tiles = vec![];
    for x in 0..size.0 {
        if x <= 6 || x >= landing_x {
            let top = if x >= landing_x { 1 + height } else { 1 };
            for y in 0..=top {
                tiles.push(((reflect(x), y), TileKind::Solid));
            }
        }
    }
    tiles.push((
        (reflect(if runup { 3 } else { 5 }), 3),
        TileKind::PlayerSpawn,
    ));
    let mut map = Map::synthetic(size, tiles);
    map.simulation_seed = Some(0);
    let margin = map.boundary_margin;
    let mut app = build_app(map);
    app.world_mut()
        .resource_mut::<ScriptEditorState>()
        .controls_enabled = true;
    let direction = if left { -1.0 } else { 1.0 };
    let key = if left {
        KeyCode::ArrowLeft
    } else {
        KeyCode::ArrowRight
    };
    let tile = app.world().resource::<StageGridMetrics>().local_tile_size.x;
    let to_source_x = |p: Vec2| {
        let x = (p.x + 900.0) / tile - margin.0 as f32;
        if left { size.0 as f32 - x } else { x }
    };
    let mut origin = Vec2::ZERO;
    let mut takeoff = None;
    let mut peak: f32 = 0.0;
    let mut travel: f32 = 0.0;
    let mut stable = 0;
    let mut landed = false;
    let mut jumped = false;
    let mut standing_since = None;
    for frame in 0..420 {
        let pos = player_position(&mut app).ok_or("No reach player")?;
        if frame == 30 {
            origin = pos;
        }
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        if frame >= 30 {
            keys.press(key);
            keys.release(KeyCode::Space);
            if takeoff.is_none() && to_source_x(pos) >= takeoff_cell {
                let ready = if runup {
                    true
                } else {
                    keys.release(key);
                    let arrived = *standing_since.get_or_insert(frame);
                    frame >= arrived + 30
                };
                if ready {
                    keys.press(key);
                    keys.press(KeyCode::Space);
                    takeoff = Some(pos);
                    origin = pos;
                }
            }
            // Stop after moving safely onto the destination, then observe landing.
            if to_source_x(pos) >= landing_x as f32 + 1.0 {
                keys.release(key);
            }
        }
        app.update();
        if takeoff.is_some() {
            let pos = player_position(&mut app).unwrap();
            peak = peak.max(pos.y - origin.y);
            travel = travel.max((pos.x - origin.x) * direction);
            let world = app.world_mut();
            let velocity = world
                .query_filtered::<&LinearVelocity, With<Player>>()
                .iter(world)
                .next()
                .unwrap()
                .0;
            jumped |= velocity.y > 1.0;
            let at_height = (pos.y - origin.y - height as f32 * tile).abs() < tile * 0.25;
            if jumped
                && to_source_x(pos) >= landing_x as f32 + 0.4
                && at_height
                && velocity.y.abs() < 1.0
            {
                stable += 1;
            } else {
                stable = 0;
            }
            if stable >= 6 {
                landed = true;
                break;
            }
        }
    }
    Ok(Trial {
        requested_takeoff_cell: takeoff_cell,
        actual_takeoff_cell: takeoff.map(to_source_x),
        kind: if step { "step" } else { "gap" },
        cells,
        px: cells as f32 * tile,
        landed,
        jumped,
        peak_rise_px: peak,
        horizontal_travel_px: travel,
    })
}
pub(super) fn measure() -> Result<ReachReport> {
    let mut measurements = vec![];
    for size in [(26, 10), (28, 10), (28, 18)] {
        for left in [false, true] {
            for runup in [false, true] {
                let mut trials = vec![];
                // Include a zero-size control. Keep failures rather than assuming monotonicity.
                for step in [true, false] {
                    for cells in 0..=if step { 5 } else { 8 } {
                        for takeoff in [5.5, 6.0, 6.5, 6.75, 6.9] {
                            trials.push(trial(size, left, runup, step, cells, takeoff)?);
                        }
                    }
                }
                let maximum = |kind| {
                    trials
                        .iter()
                        .filter(|t| t.kind == kind && t.landed)
                        .map(|t| t.cells)
                        .max()
                        .unwrap_or(-1)
                };
                let max_step = maximum("step");
                let max_gap = maximum("gap");
                measurements.push(Measurement {
                    map_size: [size.0, size.1],
                    direction: if left { "left" } else { "right" },
                    approach: if runup { "running" } else { "standing" },
                    max_step_cells: max_step,
                    max_step_px: max_step as f32 * 60.0,
                    max_gap_cells: max_gap,
                    max_gap_px: max_gap as f32 * 60.0,
                    trials,
                });
            }
        }
    }
    let hypothesis = (0..4).all(|i| {
        (1..3).all(|m| {
            measurements[i].max_step_cells >= 0
                && measurements[i].max_gap_cells >= 0
                && measurements[i].max_step_cells == measurements[m * 4 + i].max_step_cells
                && measurements[i].max_gap_cells == measurements[m * 4 + i].max_gap_cells
        })
    });
    Ok(ReachReport {
        schema_version: 1,
        viewport_px: [1800.0, 1200.0],
        tile_px: 60.0,
        hypothesis_equal_cell_reach: hypothesis,
        protocol: "60Hz; 30 settle frames; one jump at source coordinates 5.5/6/6.5/6.75/6.9 (ledge at 7); run-up from cell 3, or pre-position from cell 5 and stand for 30 frames; hold direction until one cell onto destination; six stable landing frames; 420-frame limit. Step candidates 0..5, gap candidates 0..8. Bounds apply only to this explicit input protocol, not an exhaustive reachability search. map_size is the interior; product canvas remains 30x20.",
        measurements,
    })
}
