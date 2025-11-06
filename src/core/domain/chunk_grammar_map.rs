use rand::{Rng, seq::SliceRandom};
use std::collections::{HashMap, HashSet};
use std::io::BufReader;

use serde::Deserialize;

const MAP_SIZE: (isize, isize) = (30, 20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dir {
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

#[derive(Clone, Copy, Debug)]
struct Port {
    x: isize,
    y: isize,
    dir: Dir, // チャンク外へ出る（または入る）向き
}

#[derive(Clone, Copy, Debug)]
enum TileKind {
    Solid,
    PlayerSpawn,
    Goal,
}

#[derive(Clone, Copy, Debug)]
struct Tile {
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
}

#[derive(Clone, Debug)]
struct PlacedChunk {
    id: String,
    exits_world: Vec<((isize, isize), Dir)>, // 位置＋方向
    tiles_world: Vec<Tile>,
}

#[derive(Debug, Deserialize)]
struct ChunkTemplate {
    id: String,
    map: Vec<String>,
}

impl ChunkTemplate {
    fn to_inner_template(&self) -> InnerChunkTemplate {
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

        InnerChunkTemplate {
            id: self.id.clone(),
            size: (width, height),
            entry: entry.expect("entry point 'I' not found"),
            exits,
            tiles,
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
struct ChunkGrammarConfig(StartChunks, MiddleChunks, GoalChunks);

impl ChunkGrammarConfig {
    fn starts(&self) -> Vec<InnerChunkTemplate> {
        self.0
            .templates
            .iter()
            .map(|t| t.to_inner_template())
            .collect()
    }

    fn middles(&self) -> Vec<InnerChunkTemplate> {
        self.1
            .templates
            .iter()
            .map(|t| t.to_inner_template())
            .collect()
    }

    fn goals(&self) -> Vec<InnerChunkTemplate> {
        self.2
            .templates
            .iter()
            .map(|t| t.to_inner_template())
            .collect()
    }
}

pub fn main() {
    let mut rng = rand::rng();

    let tutorial_chunk_file =
        std::fs::File::open("assets/chunk_grammar_map/tutorial.ron").expect("not found file");
    let reader = BufReader::new(tutorial_chunk_file);
    let config: ChunkGrammarConfig = ron::de::from_reader(reader).expect("failed to parse RON");

    let placed_chunks = try_build_random_path(
        &mut rng,
        &config.starts(),
        &config.middles(),
        &config.goals(),
    );

    println!("== Placed Chunks ==");
    for chunk in &placed_chunks {
        println!("- {}", chunk.id);
    }
    println!();

    println!("== ASCII Map ==");
    let map = build_tile_char_map(&placed_chunks);
    print_ascii_map(&map);
}

fn build_tile_char_map(placed_chunks: &[PlacedChunk]) -> HashMap<(isize, isize), char> {
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

fn try_build_random_path(
    rng: &mut impl Rng,
    start_chunks: &[InnerChunkTemplate],
    mid_chunks: &[InnerChunkTemplate],
    goal_chunks: &[InnerChunkTemplate],
) -> Vec<PlacedChunk> {
    let placed_start = place_chunk(
        &start_chunks[rng.random_range(0..start_chunks.len())],
        (0, 0),
    );
    let start_exit = pick_exit_dir(&placed_start, Dir::Right).unwrap();

    loop {
        let goal_template = &goal_chunks[rng.random_range(0..goal_chunks.len())];
        let Some(goal_target) = random_goal_target(rng, start_exit, goal_template) else {
            continue;
        };
        println!(
            "Trying goal at origin {:?}, entry {:?}",
            goal_target.origin, goal_target.entry
        );
        if let Some(mut mid_chunks) =
            find_path_to_goal(rng, mid_chunks, start_exit, goal_target.entry)
        {
            let final_exit = mid_chunks
                .last()
                .and_then(|chunk| pick_exit_dir(chunk, Dir::Right))
                .unwrap_or(start_exit);
            if final_exit.0 != goal_target.entry {
                continue;
            }

            let mut layout = Vec::with_capacity(mid_chunks.len() + 2);
            layout.push(placed_start.clone());
            layout.append(&mut mid_chunks);
            layout.push(place_chunk(goal_template, goal_target.origin));
            return layout;
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
    let max_origin_x = MAP_SIZE.0.checked_sub(goal_template.size.0)?;
    let max_origin_y = MAP_SIZE.1.checked_sub(goal_template.size.1)?;
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
        let placed = place_next(template, Dir::Left, current_exit);
        if placed
            .tiles_world
            .iter()
            .any(|tile| tile.x < 0 || tile.x >= MAP_SIZE.0 || tile.y < 0 || tile.y >= MAP_SIZE.1)
        {
            continue;
        }
        let Some(next_exit) = pick_exit_dir(&placed, Dir::Right) else {
            continue;
        };
        let (next_pos, _) = next_exit;
        if next_pos.0 > goal_entry.0 {
            continue;
        }
        if next_pos.1 >= MAP_SIZE.1 {
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

fn print_ascii_map(map: &HashMap<(isize, isize), char>) {
    let (map_width, map_height) = MAP_SIZE;
    for y in (0..map_height).rev() {
        for x in 0..map_width {
            let ch = map.get(&(x, y)).copied().unwrap_or('.');
            print!("{ch}");
        }
        println!();
    }
}
