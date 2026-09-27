use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use rand::{RngExt, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::Deserialize;

pub const CANVAS_SIZE: (i32, i32) = (30, 20);

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub enum StoneType {
    #[default]
    Type1,
    Type2,
    Type3,
    Type4,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Adjustments {
    pub stones: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StageConfig {
    pub map_size: (i32, i32),
    #[serde(default)]
    pub stone_type: StoneType,
    pub dig_limit: Option<u32>,
    #[serde(default)]
    pub dynamic_max: u32,
    #[serde(default)]
    pub dynamic_min: u32,
    pub adjustments: Option<Adjustments>,
    start_chunks: Vec<ChunkTemplate>,
    middle_chunks: Vec<ChunkTemplate>,
    goal_chunks: Vec<ChunkTemplate>,
}

#[derive(Clone, Debug, Deserialize)]
struct ChunkTemplate {
    id: String,
    map: Vec<String>,
    #[serde(default)]
    required_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    Solid,
    Player,
    Stone,
    Goal,
    Wall,
    Obstacle,
    DynamicSolid,
    Placed,
}

impl TileKind {
    pub fn symbol(self) -> char {
        match self {
            Self::Solid => '*',
            Self::Player => '@',
            Self::Stone => 'S',
            Self::Goal => 'G',
            Self::Wall => '#',
            Self::Obstacle => 'O',
            Self::DynamicSolid => '?',
            Self::Placed => '+',
        }
    }

    pub fn blocks(self) -> bool {
        matches!(
            self,
            Self::Solid | Self::Wall | Self::Obstacle | Self::DynamicSolid | Self::Placed
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Dir {
    #[default]
    Left,
    Right,
}

impl Dir {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Port {
    x: i32,
    y: i32,
    dir: Dir,
}

type ExitPoint = ((i32, i32), Dir);

#[derive(Clone, Copy, Debug)]
struct Tile {
    x: i32,
    y: i32,
    kind: TileKind,
}

#[derive(Clone, Debug)]
struct InnerChunk {
    id: String,
    size: (i32, i32),
    entry: Port,
    exits: Vec<Port>,
    tiles: Vec<Tile>,
    required_count: usize,
}

#[derive(Clone, Debug)]
struct PlacedChunk {
    id: String,
    exits: Vec<ExitPoint>,
    tiles: Vec<Tile>,
}

#[derive(Clone, Debug)]
pub struct GeneratedMap {
    pub canvas_size: (i32, i32),
    pub design_size: (i32, i32),
    pub boundary_margin: (i32, i32),
    pub stone_type: StoneType,
    pub dig_limit: Option<u32>,
    pub dynamic_min: u32,
    pub dynamic_max: u32,
    pub stone_adjustments: Vec<(f32, f32)>,
    pub selected_chunks: Vec<String>,
    pub tiles: HashMap<(i32, i32), TileKind>,
}

impl GeneratedMap {
    pub fn positions(&self, kind: TileKind) -> Vec<(i32, i32)> {
        let mut positions = self
            .tiles
            .iter()
            .filter_map(|(position, value)| (*value == kind).then_some(*position))
            .collect::<Vec<_>>();
        positions.sort_by_key(|(x, y)| (*y, *x));
        positions
    }
}

pub fn parse_stage(input: &str) -> Result<StageConfig> {
    Ok(ron::de::from_str(input)?)
}

pub fn generate(config: &StageConfig, seed: u64) -> Result<GeneratedMap> {
    if config.start_chunks.is_empty() || config.goal_chunks.is_empty() {
        bail!("stage must contain at least one start and goal chunk");
    }

    let starts = config
        .start_chunks
        .iter()
        .map(|chunk| to_inner(chunk, false))
        .collect::<Result<Vec<_>>>()?;
    let middles = config
        .middle_chunks
        .iter()
        .map(|chunk| to_inner(chunk, true))
        .collect::<Result<Vec<_>>>()?;
    let goals = config
        .goal_chunks
        .iter()
        .map(|chunk| to_inner(chunk, true))
        .collect::<Result<Vec<_>>>()?;

    let mut rng = StdRng::seed_from_u64(seed);
    let mut placed = build_layout(config.map_size, &starts, &middles, &goals, &mut rng)?;
    let margin = (
        (CANVAS_SIZE.0 - config.map_size.0) / 2,
        (CANVAS_SIZE.1 - config.map_size.1) / 2,
    );

    for chunk in &mut placed {
        for ((x, y), _) in &mut chunk.exits {
            *x += margin.0;
            *y += margin.1;
        }
        for tile in &mut chunk.tiles {
            tile.x += margin.0;
            tile.y += margin.1;
        }
    }

    adjust_goal_layout(&mut placed, margin.1);

    let mut tiles = build_margin_tiles(margin);
    for chunk in &placed {
        for tile in &chunk.tiles {
            tiles.insert((tile.x, tile.y), tile.kind);
        }
    }

    Ok(GeneratedMap {
        canvas_size: CANVAS_SIZE,
        design_size: config.map_size,
        boundary_margin: margin,
        stone_type: config.stone_type,
        dig_limit: config.dig_limit,
        dynamic_min: config.dynamic_min,
        dynamic_max: config.dynamic_max,
        stone_adjustments: config
            .adjustments
            .as_ref()
            .map(|value| value.stones.clone())
            .unwrap_or_default(),
        selected_chunks: placed.iter().map(|chunk| chunk.id.clone()).collect(),
        tiles,
    })
}

fn to_inner(chunk: &ChunkTemplate, require_entry: bool) -> Result<InnerChunk> {
    let height = chunk.map.len() as i32;
    let width = chunk
        .map
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0) as i32;
    let mut entry = None;
    let mut exits = Vec::new();
    let mut tiles = Vec::new();

    for (row_index, row) in chunk.map.iter().enumerate() {
        for (column_index, character) in row.chars().enumerate() {
            let x = column_index as i32;
            let y = height - row_index as i32 - 1;
            let kind = match character {
                '#' => Some(TileKind::Solid),
                '@' => Some(TileKind::Player),
                'S' => Some(TileKind::Stone),
                'G' => Some(TileKind::Goal),
                'O' => Some(TileKind::Obstacle),
                '?' => Some(TileKind::DynamicSolid),
                'I' => {
                    entry = Some(Port {
                        x,
                        y,
                        dir: Dir::Left,
                    });
                    None
                }
                'E' => {
                    exits.push(Port {
                        x,
                        y,
                        dir: Dir::Right,
                    });
                    None
                }
                _ => None,
            };
            if let Some(kind) = kind {
                tiles.push(Tile { x, y, kind });
            }
        }
    }

    let entry = match (require_entry, entry) {
        (true, None) => bail!("chunk '{}' has no I entry", chunk.id),
        (_, Some(value)) => value,
        (false, None) => Port::default(),
    };

    Ok(InnerChunk {
        id: chunk.id.clone(),
        size: (width, height),
        entry,
        exits,
        tiles,
        required_count: chunk.required_count,
    })
}

fn build_layout(
    map_size: (i32, i32),
    starts: &[InnerChunk],
    middles: &[InnerChunk],
    goals: &[InnerChunk],
    rng: &mut StdRng,
) -> Result<Vec<PlacedChunk>> {
    let start = place_chunk(&starts[rng.random_range(0..starts.len())], (0, 0));
    let Some(start_exit) = pick_exit(&start) else {
        bail!("selected start chunk '{}' has no E exit", start.id);
    };

    let mut required = Vec::new();
    let mut optional = Vec::new();
    for middle in middles {
        if middle.required_count > 0 {
            for _ in 0..middle.required_count {
                required.push(middle);
            }
        } else {
            optional.push(middle);
        }
    }

    for _ in 0..10_000 {
        let mut required_queue = required.clone();
        required_queue.shuffle(rng);
        let mut current_exit = start_exit;
        let mut required_placed = Vec::new();
        let mut failed = false;

        for template in required_queue {
            let ((x, y), _) = current_exit;
            if x < template.entry.x || y < template.entry.y {
                failed = true;
                break;
            }
            let Some((placed, next_exit)) = place_middle(template, current_exit, map_size) else {
                failed = true;
                break;
            };
            current_exit = next_exit;
            required_placed.push(placed);
        }
        if failed {
            continue;
        }

        let goal = &goals[rng.random_range(0..goals.len())];
        let Some(goal_target) = goal_target(rng, map_size, current_exit, goal) else {
            continue;
        };
        let Some(mut path) = find_path(rng, map_size, &optional, current_exit, goal_target.entry)
        else {
            continue;
        };
        let final_exit = path.last().and_then(pick_exit).unwrap_or(current_exit).0;
        if final_exit != goal_target.entry {
            continue;
        }

        let mut layout = Vec::with_capacity(required_placed.len() + path.len() + 2);
        layout.push(start.clone());
        layout.extend(required_placed);
        layout.append(&mut path);
        layout.push(place_chunk(goal, goal_target.origin));
        return Ok(layout);
    }

    bail!("could not build a valid chunk path after 10000 attempts")
}

fn pick_exit(chunk: &PlacedChunk) -> Option<ExitPoint> {
    chunk
        .exits
        .iter()
        .copied()
        .find(|(_, direction)| *direction == Dir::Right)
}

fn place_chunk(chunk: &InnerChunk, origin: (i32, i32)) -> PlacedChunk {
    PlacedChunk {
        id: chunk.id.clone(),
        exits: chunk
            .exits
            .iter()
            .map(|port| ((origin.0 + port.x, origin.1 + port.y), port.dir))
            .collect(),
        tiles: chunk
            .tiles
            .iter()
            .map(|tile| Tile {
                x: origin.0 + tile.x,
                y: origin.1 + tile.y,
                kind: tile.kind,
            })
            .collect(),
    }
}

fn place_middle(
    chunk: &InnerChunk,
    exit: ExitPoint,
    map_size: (i32, i32),
) -> Option<(PlacedChunk, ExitPoint)> {
    if chunk.entry.dir.opposite() != exit.1 {
        return None;
    }
    let origin = (exit.0.0 - chunk.entry.x, exit.0.1 - chunk.entry.y);
    let placed = place_chunk(chunk, origin);
    if placed
        .tiles
        .iter()
        .any(|tile| tile.x < 0 || tile.x >= map_size.0 || tile.y < 0 || tile.y >= map_size.1)
    {
        return None;
    }
    let next = pick_exit(&placed)?;
    (next.0.1 < map_size.1).then_some((placed, next))
}

struct GoalTarget {
    origin: (i32, i32),
    entry: (i32, i32),
}

fn goal_target(
    rng: &mut StdRng,
    map_size: (i32, i32),
    start_exit: ExitPoint,
    goal: &InnerChunk,
) -> Option<GoalTarget> {
    let max_x = map_size.0.checked_sub(goal.size.0)?;
    let max_y = map_size.1.checked_sub(goal.size.1)?;
    let min_x = start_exit.0.0.checked_sub(goal.entry.x)?;
    if min_x > max_x {
        return None;
    }
    let origin_y = if max_y <= 0 {
        0
    } else {
        rng.random_range(0..=max_y)
    };
    let entry = (max_x + goal.entry.x, origin_y + goal.entry.y);
    (entry.0 >= start_exit.0.0).then_some(GoalTarget {
        origin: (max_x, origin_y),
        entry,
    })
}

fn find_path(
    rng: &mut StdRng,
    map_size: (i32, i32),
    candidates: &[&InnerChunk],
    start: ExitPoint,
    goal: (i32, i32),
) -> Option<Vec<PlacedChunk>> {
    let mut path = Vec::new();
    let mut visited = HashSet::from([start.0]);
    search_path(
        rng,
        map_size,
        candidates,
        start,
        goal,
        &mut path,
        &mut visited,
    )
}

fn search_path(
    rng: &mut StdRng,
    map_size: (i32, i32),
    candidates: &[&InnerChunk],
    current: ExitPoint,
    goal: (i32, i32),
    path: &mut Vec<PlacedChunk>,
    visited: &mut HashSet<(i32, i32)>,
) -> Option<Vec<PlacedChunk>> {
    if current.0 == goal {
        return Some(path.clone());
    }
    if current.0.0 > goal.0 {
        return None;
    }

    let mut shuffled = candidates.to_vec();
    shuffled.shuffle(rng);
    for candidate in shuffled {
        if current.0.0 < candidate.entry.x || current.0.1 < candidate.entry.y {
            continue;
        }
        let Some((placed, next)) = place_middle(candidate, current, map_size) else {
            continue;
        };
        if next.0.0 > goal.0 || !visited.insert(next.0) {
            continue;
        }
        path.push(placed);
        if let Some(result) = search_path(rng, map_size, candidates, next, goal, path, visited) {
            return Some(result);
        }
        path.pop();
        visited.remove(&next.0);
    }
    None
}

fn build_margin_tiles(margin: (i32, i32)) -> HashMap<(i32, i32), TileKind> {
    let mut tiles = HashMap::new();
    for x in 0..CANVAS_SIZE.0 {
        for y in 0..CANVAS_SIZE.1 {
            if x < margin.0
                || x >= CANVAS_SIZE.0 - margin.0
                || y < margin.1
                || y >= CANVAS_SIZE.1 - margin.1
            {
                tiles.insert((x, y), TileKind::Wall);
            }
        }
    }
    tiles
}

fn adjust_goal_layout(chunks: &mut [PlacedChunk], target_bottom: i32) {
    let Some(goal_x) = chunks
        .iter()
        .flat_map(|chunk| &chunk.tiles)
        .filter(|tile| tile.kind == TileKind::Goal)
        .map(|tile| tile.x)
        .max()
    else {
        return;
    };
    let Some(min_y) = chunks
        .iter()
        .flat_map(|chunk| &chunk.tiles)
        .filter(|tile| tile.kind == TileKind::Goal && tile.x == goal_x)
        .map(|tile| tile.y)
        .min()
    else {
        return;
    };
    let Some(chunk_index) = chunks.iter().position(|chunk| {
        chunk
            .tiles
            .iter()
            .any(|tile| tile.kind == TileKind::Goal && tile.x == goal_x)
    }) else {
        return;
    };

    for y in target_bottom..min_y {
        chunks[chunk_index].tiles.push(Tile {
            x: goal_x,
            y,
            kind: TileKind::Goal,
        });
    }

    let goal_rows = chunks
        .iter()
        .flat_map(|chunk| &chunk.tiles)
        .filter(|tile| tile.kind == TileKind::Goal && tile.x == goal_x)
        .map(|tile| tile.y)
        .collect::<Vec<_>>();
    for y in goal_rows {
        let guard = (goal_x - 1, y);
        let mut found = false;
        for chunk in chunks.iter_mut() {
            for tile in &mut chunk.tiles {
                if (tile.x, tile.y) == guard {
                    tile.kind = TileKind::Solid;
                    found = true;
                }
            }
        }
        if !found {
            chunks[chunk_index].tiles.push(Tile {
                x: guard.0,
                y: guard.1,
                kind: TileKind::Solid,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_generates_fixed_stage() {
        let input = r#"(
            map_size: (28, 18),
            stone_type: Type3,
            dig_limit: Some(5),
            start_chunks: [ChunkTemplate(id: "start", map: ["@S.E"])],
            middle_chunks: [ChunkTemplate(id: "noop", map: ["IE"])],
            goal_chunks: [ChunkTemplate(id: "goal", map: ["IG"])],
        )"#;
        let map = generate(&parse_stage(input).unwrap(), 7).unwrap();
        assert_eq!(map.positions(TileKind::Player).len(), 1);
        assert_eq!(map.positions(TileKind::Stone).len(), 1);
        assert!(!map.positions(TileKind::Goal).is_empty());
    }

    #[test]
    fn same_seed_produces_same_map() {
        let input = r#"(
            map_size: (28, 18),
            stone_type: Type1,
            dig_limit: Some(0),
            start_chunks: [ChunkTemplate(id: "start", map: ["@.E"])],
            middle_chunks: [ChunkTemplate(id: "noop", map: ["IE"])],
            goal_chunks: [ChunkTemplate(id: "goal-a", map: ["IG"]), ChunkTemplate(id: "goal-b", map: ["I.G"])],
        )"#;
        let config = parse_stage(input).unwrap();
        let a = generate(&config, 42).unwrap();
        let b = generate(&config, 42).unwrap();
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.selected_chunks, b.selected_chunks);
    }
}
