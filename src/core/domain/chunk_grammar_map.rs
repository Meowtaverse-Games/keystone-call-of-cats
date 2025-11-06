use rand::{Rng, seq::SliceRandom};
use std::collections::{HashMap, HashSet};
use std::io::BufReader;

use serde::{Deserialize, Serialize};

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
    id: &'static str,
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

#[derive(Debug, Deserialize, Serialize)]
struct MyStruct {
    boolean: bool,
    float: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChunkTemplate {
    id: String,
    map: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StartChunks {
    templates: Vec<ChunkTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MiddleChunks {
    templates: Vec<ChunkTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoalChunks {
    templates: Vec<ChunkTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChunkGrammarConfig(
    StartChunks,
    MiddleChunks,
    GoalChunks,
);

pub fn main() {
    let x: MyStruct = ron::from_str("(boolean: true, float: 1.23)").unwrap();

    let tutorial_chunk_file =
        std::fs::File::open("assets/chunk_grammar_map/tutorial.ron").expect("not found file");
    let reader = BufReader::new(tutorial_chunk_file);
    let config: ChunkGrammarConfig = ron::de::from_reader(reader).expect("failed to parse RON");

    println!(
        "Pretty RON: {}",
        ron::ser::to_string_pretty(&config, ron::ser::PrettyConfig::default()).unwrap(),
    );

    let mut rng = rand::rng();

    let start = chunk_start_flat();
    let mid_chunk_factories: &[fn() -> InnerChunkTemplate] = &[
        chunk_simple_platform,
        chunk_flat_bridge,
        chunk_gap_jump,
        chunk_plateau,
        chunk_stairs_up_small,
        chunk_stairs_down_small,
    ];
    let goal_chunk_factories: &[fn() -> InnerChunkTemplate] = &[chunk_goal_platform, chunk_goal_lower];
    let placed_chunks =
        try_build_random_path(&mut rng, &start, mid_chunk_factories, goal_chunk_factories);

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
    start: &InnerChunkTemplate,
    mid_chunk_factories: &[fn() -> InnerChunkTemplate],
    goal_chunk_factories: &[fn() -> InnerChunkTemplate],
) -> Vec<PlacedChunk> {
    let placed_start = place_chunk(start, (0, 0));
    let start_exit = pick_exit_dir(&placed_start, Dir::Right).unwrap();

    loop {
        let goal_template = goal_chunk_factories[rng.random_range(0..goal_chunk_factories.len())]();
        let Some(goal_target) = random_goal_target(rng, start_exit, &goal_template) else {
            continue;
        };
        println!(
            "Trying goal at origin {:?}, entry {:?}",
            goal_target.origin, goal_target.entry
        );
        if let Some(mut mid_chunks) =
            find_path_to_goal(rng, mid_chunk_factories, start_exit, goal_target.entry)
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
            layout.push(place_chunk(&goal_template, goal_target.origin));
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
    mid_chunk_factories: &[fn() -> InnerChunkTemplate],
    start_exit: ((isize, isize), Dir),
    goal_entry: (isize, isize),
) -> Option<Vec<PlacedChunk>> {
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(start_exit.0);
    search_path_to_goal(
        rng,
        mid_chunk_factories,
        start_exit,
        goal_entry,
        &mut path,
        &mut visited,
    )
}

fn search_path_to_goal(
    rng: &mut impl Rng,
    mid_chunk_factories: &[fn() -> InnerChunkTemplate],
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

    let mut candidates: Vec<fn() -> InnerChunkTemplate> = mid_chunk_factories.to_vec();
    candidates.shuffle(rng);

    for template_fn in candidates {
        let template = template_fn();
        if current_pos.0 < template.entry.x || current_pos.1 < template.entry.y {
            continue;
        }
        let placed = place_next(&template, Dir::Left, current_exit);
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
        if let Some(result) = search_path_to_goal(
            rng,
            mid_chunk_factories,
            next_exit,
            goal_entry,
            path,
            visited,
        ) {
            return Some(result);
        }
        path.pop();
        visited.remove(&next_pos);
    }

    None
}

/* ------------------------- サンプルチャンク ------------------------- */

fn chunk_start_flat() -> InnerChunkTemplate {
    // 6x4 の小部屋。右と上に出口。地面とプレイヤー初期位置あり。
    /*
      ..E...
      ......
      .@##.E
      ######
    */
    InnerChunkTemplate {
        id: "start_flat",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        }, // スタート側の接続統一用（実際は未使用）
        exits: vec![
            Port {
                x: 5,
                y: 1,
                dir: Dir::Right,
            }, // 右へ
            Port {
                x: 2,
                y: 3,
                dir: Dir::Up,
            }, // 上へ
        ],
        tiles: {
            let mut v = vec![];
            // 地面
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            // 低い段＋プレイヤー
            v.push(Tile {
                x: 1,
                y: 1,
                kind: TileKind::PlayerSpawn,
            });
            v.push(Tile {
                x: 2,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 1,
                kind: TileKind::Solid,
            });
            v
        },
    }
}

fn chunk_simple_platform() -> InnerChunkTemplate {
    // 緩やかな傾斜で高さが少し上がるシンプルな橋。
    InnerChunkTemplate {
        id: "flat_platform",
        size: (2, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 1,
            y: 1,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            for x in 0..2 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            v
        },
    }
}

fn chunk_flat_bridge() -> InnerChunkTemplate {
    // 緩やかな傾斜で高さが少し上がるシンプルな橋。
    InnerChunkTemplate {
        id: "flat_bridge",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 5,
            y: 1,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            v.push(Tile {
                x: 2,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 2,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 4,
                y: 1,
                kind: TileKind::Solid,
            });
            v
        },
    }
}

fn chunk_gap_jump() -> InnerChunkTemplate {
    // 小さな穴と段差。真ん中をジャンプで越えるイメージ。
    InnerChunkTemplate {
        id: "gap_jump",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 5,
            y: 1,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            for x in 0..6 {
                if (2..=3).contains(&x) {
                    continue;
                }
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            v.push(Tile {
                x: 2,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 2,
                kind: TileKind::Solid,
            });
            v
        },
    }
}

fn chunk_plateau() -> InnerChunkTemplate {
    // 高さ2の台地が中央にあるチャンク。
    InnerChunkTemplate {
        id: "plateau",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 5,
            y: 1,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            for x in 2..5 {
                v.push(Tile {
                    x,
                    y: 1,
                    kind: TileKind::Solid,
                });
            }
            for x in 3..5 {
                v.push(Tile {
                    x,
                    y: 2,
                    kind: TileKind::Solid,
                });
            }
            v
        },
    }
}

fn chunk_stairs_up_small() -> InnerChunkTemplate {
    // 6x4。左から入って右上へ出る階段チャンク。さらに上方向の出口も一つ。
    InnerChunkTemplate {
        id: "stairs_up_small",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        },
        exits: vec![
            Port {
                x: 5,
                y: 2,
                dir: Dir::Right,
            }, // 右上へ
            Port {
                x: 3,
                y: 3,
                dir: Dir::Up,
            }, // 上へ
        ],
        tiles: {
            let mut v = vec![];
            // 下地
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            // 段差
            v.push(Tile {
                x: 2,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 2,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 4,
                y: 2,
                kind: TileKind::Solid,
            });
            v
        },
    }
}

fn chunk_stairs_down_small() -> InnerChunkTemplate {
    // 左が高台になっている小さな下り階段。
    InnerChunkTemplate {
        id: "stairs_down_small",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 2,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 5,
            y: 1,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            v.push(Tile {
                x: 0,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 0,
                y: 2,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 1,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 2,
                y: 1,
                kind: TileKind::Solid,
            });
            v
        },
    }
}

fn chunk_goal_platform() -> InnerChunkTemplate {
    // 6x4。左から入って中段にゴールがある足場。右にも出口。
    /*
     ......
     I....G
     ######
     ......
    */
    InnerChunkTemplate {
        id: "goal_platform",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 2,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 5,
            y: 2,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            // 中段足場
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 1,
                    kind: TileKind::Solid,
                });
            }
            v.push(Tile {
                x: 5,
                y: 2,
                kind: TileKind::Goal,
            });
            v
        },
    }
}

fn chunk_goal_lower() -> InnerChunkTemplate {
    // 6x4。低めの足場にゴールが配置されたバリエーション。
    /*
     ......
     ......
     I.##.G
     ######
    */
    InnerChunkTemplate {
        id: "goal_lower",
        size: (6, 4),
        entry: Port {
            x: 0,
            y: 1,
            dir: Dir::Left,
        },
        exits: vec![Port {
            x: 5,
            y: 1,
            dir: Dir::Right,
        }],
        tiles: {
            let mut v = vec![];
            for x in 0..6 {
                v.push(Tile {
                    x,
                    y: 0,
                    kind: TileKind::Solid,
                });
            }
            v.push(Tile {
                x: 2,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 3,
                y: 1,
                kind: TileKind::Solid,
            });
            v.push(Tile {
                x: 5,
                y: 1,
                kind: TileKind::Goal,
            });
            v
        },
    }
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
