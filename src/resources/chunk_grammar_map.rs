use rand::{Rng, seq::SliceRandom};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;

pub const MAP_SIZE: (isize, isize) = (30, 20);
const BOUNDARY_MARGIN: isize = 1;
const INNER_MAP_SIZE: (isize, isize) = (
    MAP_SIZE.0 - 2 * BOUNDARY_MARGIN,
    MAP_SIZE.1 - 2 * BOUNDARY_MARGIN,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Dir {
    #[default]
    Left,
    Right,
    Up,
    Down,
}
impl Dir {
    fn opposite(self) -> Self {
        match self {
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Port {
    x: isize,
    y: isize,
    dir: Dir, // チャンク外へ出る（または入る）向き
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileKind {
    Solid,
    PlayerSpawn,
    Goal,
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    x: isize,
    y: isize,
    kind: TileKind,
}

#[derive(Clone, Debug)]
struct InnerChunkTemplate {
    id: String,
    size: (isize, isize),
    entry: Port,
    exits: Vec<Port>,
    tiles: Vec<Tile>,
    required_count: usize,
}

#[derive(Clone, Debug)]
pub struct PlacedChunk {
    pub id: String,
    exits_world: Vec<((isize, isize), Dir)>, // 位置＋方向
    pub tiles_world: Vec<Tile>,
}

#[derive(Debug, Deserialize)]
struct ChunkTemplate {
    id: String,
    map: Vec<String>,
    #[serde(default)]
    required_count: usize,
}

impl ChunkTemplate {
    fn to_inner_template(&self, check_entry: bool) -> InnerChunkTemplate {
        let height = self.map.len() as isize;
        let width = self.map.iter().map(|row| row.len()).max().unwrap_or(0) as isize;

        let mut tiles = Vec::new();
        let mut entry = None;
        let mut exits = Vec::new();

        for (y, row) in self.map.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let x = x as isize;
                let y = height - 1 - y as isize; // 上下反転
                match ch {
                    '#' => tiles.push(Tile {
                        x,
                        y,
                        kind: TileKind::Solid,
                    }),
                    '@' => tiles.push(Tile {
                        x,
                        y,
                        kind: TileKind::PlayerSpawn,
                    }),
                    'G' => tiles.push(Tile {
                        x,
                        y,
                        kind: TileKind::Goal,
                    }),
                    'I' => {
                        entry = Some(Port {
                            x,
                            y,
                            dir: Dir::Left, // 仮
                        });
                    }
                    'E' => exits.push(Port {
                        x,
                        y,
                        dir: Dir::Right, // 仮
                    }),
                    _ => {}
                }
            }
        }

        let entry = if check_entry {
            entry.expect("entry point 'I' not found")
        } else {
            Port {
                x: 0,
                y: 0,
                dir: Dir::Left,
            }
        };

        InnerChunkTemplate {
            id: self.id.clone(),
            size: (width, height),
            entry,
            exits,
            tiles,
            required_count: self.required_count,
        }
    }
}

#[derive(Debug, Deserialize)]
struct StartChunks {
    templates: Vec<ChunkTemplate>,
}

#[derive(Debug, Deserialize)]
struct MiddleChunks {
    templates: Vec<ChunkTemplate>,
}

#[derive(Debug, Deserialize)]
struct GoalChunks {
    templates: Vec<ChunkTemplate>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkGrammarConfig(StartChunks, MiddleChunks, GoalChunks);

impl ChunkGrammarConfig {
    fn starts(&self) -> Vec<InnerChunkTemplate> {
        self.0
            .templates
            .iter()
            .map(|t| t.to_inner_template(false))
            .collect()
    }

    fn middles(&self) -> Vec<InnerChunkTemplate> {
        self.1
            .templates
            .iter()
            .map(|t| t.to_inner_template(false))
            .collect()
    }

    fn goals(&self) -> Vec<InnerChunkTemplate> {
        self.2
            .templates
            .iter()
            .map(|t| t.to_inner_template(false))
            .collect()
    }
}

#[derive(Debug)]
pub enum ChunkGrammarError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
}

impl fmt::Display for ChunkGrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkGrammarError::Io(err) => write!(f, "io error: {err}"),
            ChunkGrammarError::Parse(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl std::error::Error for ChunkGrammarError {}

pub fn load_config_from_file(
    path: impl AsRef<Path>,
) -> Result<ChunkGrammarConfig, ChunkGrammarError> {
    let file = File::open(path).map_err(ChunkGrammarError::Io)?;
    let reader = BufReader::new(file);
    ron::de::from_reader(reader).map_err(ChunkGrammarError::Parse)
}

pub fn generate_random_layout(config: &ChunkGrammarConfig) -> PlacedChunkLayout {
    let starts = config.starts();
    let middles = config.middles();
    let goals = config.goals();
    try_build_random_path(&starts, &middles, &goals)
}

pub fn generate_random_layout_from_file(
    path: impl AsRef<Path>,
) -> Result<PlacedChunkLayout, ChunkGrammarError> {
    let config = load_config_from_file(path)?;
    Ok(generate_random_layout(&config))
}

pub fn show_ascii_map() {
    let placed_chunks = generate_random_layout_from_file("assets/stages/stage-1.ron")
        .expect("failed to generate layout from config");
    println!("== Placed Chunks ==");
    for chunk in &placed_chunks {
        println!("- {}", chunk.id);
    }
    println!();

    println!("== ASCII Map ==");
    print_ascii_map(&placed_chunks);
}

fn build_tile_char_map(placed_chunks: &PlacedChunkLayout) -> HashMap<(isize, isize), char> {
    let mut map = HashMap::<(isize, isize), char>::new();

    for chunk in placed_chunks {
        for tile in &chunk.tiles_world {
            let ch = match tile.kind {
                TileKind::Solid => '#',
                TileKind::PlayerSpawn => '@',
                TileKind::Goal => 'G',
            };
            map.insert((tile.x, tile.y), ch);
        }
    }

    map
}

/// 指定方向の出口を1つ拾う（最小実装：最初の一致を返す）
fn pick_exit_dir(p: &PlacedChunk, want: Dir) -> Option<((isize, isize), Dir)> {
    p.exits_world.iter().copied().find(|(_, d)| *d == want)
}

/// 既存の“出口（ワールド座標）”に、次チャンクの“entry（ローカル）”を合わせる
fn place_next(
    template: &InnerChunkTemplate,
    required_entry_dir: Dir,
    exit: ((isize, isize), Dir),
) -> PlacedChunk {
    assert_eq!(
        template.entry.dir, required_entry_dir,
        "entryの向きが合っていません"
    );
    let (exit_pos, exit_dir) = exit;
    // entry.dir と exit.dir は反対向きが正しい
    assert_eq!(
        required_entry_dir.opposite(),
        exit_dir,
        "entry/exit の向きが逆になっていません"
    );

    // 原点 = exit_world - entry_local
    let origin = (exit_pos.0 - template.entry.x, exit_pos.1 - template.entry.y);
    place_chunk(template, origin)
}

/// チャンクをワールドに敷く（原点のみ指定）
fn place_chunk(t: &InnerChunkTemplate, origin: (isize, isize)) -> PlacedChunk {
    let exits_world = t
        .exits
        .iter()
        .map(|p| ((origin.0 + p.x, origin.1 + p.y), p.dir))
        .collect::<Vec<_>>();
    let tiles_world = t
        .tiles
        .iter()
        .map(|tile| Tile {
            x: origin.0 + tile.x,
            y: origin.1 + tile.y,
            kind: tile.kind,
        })
        .collect::<Vec<_>>();

    PlacedChunk {
        id: t.id.to_string(),
        exits_world,
        tiles_world,
    }
}

pub struct PlacedChunkLayout {
    pub placed_chunks: Vec<PlacedChunk>,
}

impl IntoIterator for PlacedChunkLayout {
    type Item = PlacedChunk;
    type IntoIter = std::vec::IntoIter<PlacedChunk>;

    fn into_iter(self) -> Self::IntoIter {
        self.placed_chunks.into_iter()
    }
}

impl<'a> IntoIterator for &'a PlacedChunkLayout {
    type Item = &'a PlacedChunk;
    type IntoIter = std::slice::Iter<'a, PlacedChunk>;

    fn into_iter(self) -> Self::IntoIter {
        self.placed_chunks.iter()
    }
}

impl PlacedChunkLayout {
    pub fn tile_position(&self, kind: TileKind) -> Option<(isize, isize)> {
        for chunk in &self.placed_chunks {
            for tile in &chunk.tiles_world {
                if tile.kind == kind {
                    return Some((tile.x, tile.y));
                }
            }
        }
        None
    }

    pub fn map_iter(&self) -> impl Iterator<Item = ((isize, isize), TileKind)> + '_ {
        self.placed_chunks.iter().flat_map(|chunk| {
            chunk
                .tiles_world
                .iter()
                .map(|tile| ((tile.x, tile.y), tile.kind))
        })
    }
}

fn try_build_random_path(
    start_chunks: &[InnerChunkTemplate],
    mid_chunks: &[InnerChunkTemplate],
    goal_chunks: &[InnerChunkTemplate],
) -> PlacedChunkLayout {
    let mut rng = rand::rng();
    let placed_start = place_chunk(
        &start_chunks[rng.random_range(0..start_chunks.len())],
        (0, 0),
    );
    let start_exit = pick_exit_dir(&placed_start, Dir::Right).unwrap();

    let required_templates: Vec<&InnerChunkTemplate> = mid_chunks
        .iter()
        .flat_map(|template| std::iter::repeat(template).take(template.required_count))
        .collect();

    print!("required_templates: ");
    for t in &required_templates {
        print!("{} ", t.id);
    }
    println!();

    loop {
        let mut mandatory_queue = required_templates.clone();
        mandatory_queue.shuffle(&mut rng);

        let mut mandatory_chunks = Vec::with_capacity(mandatory_queue.len());
        let mut path_start_exit = start_exit;
        let mut mandatory_failed = false;
        for template in mandatory_queue {
            let (current_pos, _) = path_start_exit;
            if current_pos.0 < template.entry.x || current_pos.1 < template.entry.y {
                mandatory_failed = true;
                break;
            }
            let Some((placed, next_exit)) = place_middle_chunk(template, path_start_exit) else {
                mandatory_failed = true;
                break;
            };
            path_start_exit = next_exit;
            mandatory_chunks.push(placed);
        }
        if mandatory_failed {
            continue;
        }

        let goal_template = &goal_chunks[rng.random_range(0..goal_chunks.len())];
        let Some(goal_target) = random_goal_target(&mut rng, path_start_exit, goal_template) else {
            continue;
        };

        if let Some(mut mid_path) =
            find_path_to_goal(&mut rng, mid_chunks, path_start_exit, goal_target.entry)
        {
            let final_exit = mid_path
                .last()
                .and_then(|chunk| pick_exit_dir(chunk, Dir::Right))
                .unwrap_or(path_start_exit);
            if final_exit.0 != goal_target.entry {
                continue;
            }

            let mut layout = Vec::with_capacity(mandatory_chunks.len() + mid_path.len() + 2);
            layout.push(placed_start.clone());
            layout.extend(mandatory_chunks.into_iter());
            layout.append(&mut mid_path);
            layout.push(place_chunk(goal_template, goal_target.origin));
            return PlacedChunkLayout {
                placed_chunks: layout,
            };
        }
    }
}

struct GoalTarget {
    origin: (isize, isize),
    entry: (isize, isize),
}

fn random_goal_target(
    rng: &mut impl Rng,
    start_exit: ((isize, isize), Dir),
    goal_template: &InnerChunkTemplate,
) -> Option<GoalTarget> {
    let (start_pos, _) = start_exit;
    let max_origin_x = INNER_MAP_SIZE.0.checked_sub(goal_template.size.0)?;
    let max_origin_y = INNER_MAP_SIZE.1.checked_sub(goal_template.size.1)?;
    let min_origin_x = start_pos.0.checked_sub(goal_template.entry.x)?;
    if min_origin_x > max_origin_x {
        return None;
    }
    let origin_x = max_origin_x;
    if origin_x < min_origin_x {
        return None;
    }
    let origin_y = if max_origin_y == 0 {
        0
    } else {
        rng.random_range(0..=(max_origin_y as i32)) as isize
    };
    let entry = (
        origin_x + goal_template.entry.x,
        origin_y + goal_template.entry.y,
    );
    if entry.0 < start_pos.0 {
        return None;
    }
    Some(GoalTarget {
        origin: (origin_x, origin_y),
        entry,
    })
}

fn place_middle_chunk(
    template: &InnerChunkTemplate,
    current_exit: ((isize, isize), Dir),
) -> Option<(PlacedChunk, ((isize, isize), Dir))> {
    let placed = place_next(template, Dir::Left, current_exit);
    if placed.tiles_world.iter().any(|tile| {
        tile.x < 0 || tile.x >= INNER_MAP_SIZE.0 || tile.y < 0 || tile.y >= INNER_MAP_SIZE.1
    }) {
        return None;
    }
    let Some(next_exit) = pick_exit_dir(&placed, Dir::Right) else {
        return None;
    };
    let (next_pos, _) = next_exit;
    if next_pos.1 >= INNER_MAP_SIZE.1 {
        return None;
    }
    Some((placed, next_exit))
}

fn find_path_to_goal(
    rng: &mut impl Rng,
    mid_chunks: &[InnerChunkTemplate],
    start_exit: ((isize, isize), Dir),
    goal_entry: (isize, isize),
) -> Option<Vec<PlacedChunk>> {
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(start_exit.0);
    search_path_to_goal(
        rng,
        mid_chunks,
        start_exit,
        goal_entry,
        &mut path,
        &mut visited,
    )
}

fn search_path_to_goal(
    rng: &mut impl Rng,
    candidates: &[InnerChunkTemplate],
    current_exit: ((isize, isize), Dir),
    goal_entry: (isize, isize),
    path: &mut Vec<PlacedChunk>,
    visited: &mut HashSet<(isize, isize)>,
) -> Option<Vec<PlacedChunk>> {
    let (current_pos, _) = current_exit;
    if current_pos == goal_entry {
        return Some(path.clone());
    }
    if current_pos.0 > goal_entry.0 {
        return None;
    }

    let mut shuffled_candidates = candidates.to_vec();
    shuffled_candidates.shuffle(rng);

    for template in &shuffled_candidates {
        if current_pos.0 < template.entry.x || current_pos.1 < template.entry.y {
            continue;
        }
        let Some((placed, next_exit)) = place_middle_chunk(template, current_exit) else {
            continue;
        };
        let (next_pos, _) = next_exit;
        if next_pos.0 > goal_entry.0 {
            continue;
        }
        if !visited.insert(next_pos) {
            continue;
        }
        path.push(placed);
        if let Some(result) =
            search_path_to_goal(rng, candidates, next_exit, goal_entry, path, visited)
        {
            return Some(result);
        }
        path.pop();
        visited.remove(&next_pos);
    }

    None
}

pub fn print_ascii_map(placed_chunks: &PlacedChunkLayout) {
    let map = build_tile_char_map(placed_chunks);
    let (map_width, map_height) = INNER_MAP_SIZE;
    for y in (0..map_height).rev() {
        for x in 0..map_width {
            let ch = map.get(&(x, y)).copied().unwrap_or('.');
            print!("{ch}");
        }
        println!();
    }
}
