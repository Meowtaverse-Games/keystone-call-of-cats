use rand::Rng;
use std::collections::HashMap;

const MAP_WIDTH: i32 = 30;
const MAP_HEIGHT: i32 = 20;

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
    x: i32,
    y: i32,
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
    x: i32,
    y: i32,
    kind: TileKind,
}

/// チャンク＝小さな局所マップ。entry（入口）と複数exits（出口）を持つ。
#[derive(Clone, Debug)]
struct ChunkTemplate {
    id: &'static str,
    size: (i32, i32), // (width, height)
    entry: Port,
    exits: Vec<Port>,
    tiles: Vec<Tile>,
}

/// 実際にワールドのどこに置かれたか
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct PlacedChunk {
    id: String,
    origin: (i32, i32), // 左下原点
    size: (i32, i32),
    entry_world: (i32, i32),
    exits_world: Vec<((i32, i32), Dir)>, // 位置＋方向
    tiles_world: Vec<Tile>,
}

pub fn main() {
    let mut rng = rand::rng();

    // チャンク候補を用意。入口向きはすべて Left で統一。
    let start = chunk_start_flat();
    let mid_chunk_factories: &[fn() -> ChunkTemplate] = &[
        chunk_flat_bridge,
        chunk_gap_jump,
        chunk_plateau,
        chunk_stairs_up_small,
        chunk_stairs_down_small,
    ];
    let goal_chunk_factories: &[fn() -> ChunkTemplate] = &[chunk_goal_platform, chunk_goal_lower];

    // 左から右へ最大5チャンク（start + 0~3 mid + goal）を連結する
    let max_mid_chunks: usize = 3;
    let mid_count = if mid_chunk_factories.is_empty() {
        0
    } else {
        rng.random_range(0..=max_mid_chunks)
    };

    let mut placed_chunks = Vec::<PlacedChunk>::new();
    let placed_start = place_chunk(&start, (0, 0));
    let mut current_exit =
        pick_exit_dir(&placed_start, Dir::Right).expect("start に Right 出口が必要");
    placed_chunks.push(placed_start);

    for _ in 0..mid_count {
        let template_fn = mid_chunk_factories[rng.random_range(0..mid_chunk_factories.len())];
        let template = template_fn();
        let template_id = template.id;
        let placed = place_next(&template, Dir::Left, current_exit);
        current_exit = pick_exit_dir(&placed, Dir::Right)
            .unwrap_or_else(|| panic!("{template_id} に Right 出口が必要"));
        placed_chunks.push(placed);
    }

    let goal_template = goal_chunk_factories[rng.random_range(0..goal_chunk_factories.len())]();
    let placed_goal = place_next(&goal_template, Dir::Left, current_exit);
    placed_chunks.push(placed_goal);

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    for chunk in &placed_chunks {
        for tile in &chunk.tiles_world {
            min_x = min_x.min(tile.x);
            max_x = max_x.max(tile.x);
            min_y = min_y.min(tile.y);
            max_y = max_y.max(tile.y);
        }
    }

    if min_x == i32::MAX {
        println!("[empty]");
        return;
    }

    let width_span = (max_x - min_x + 1).max(0);
    let height_span = (max_y - min_y + 1).max(0);

    assert!(
        width_span <= MAP_WIDTH,
        "生成されたチャンク幅 {width_span} が許可サイズ {MAP_WIDTH} を超えています"
    );
    assert!(
        height_span <= MAP_HEIGHT,
        "生成されたチャンク高さ {height_span} が許可サイズ {MAP_HEIGHT} を超えています"
    );

    let offset_x = -min_x;
    let offset_y = -min_y;

    let mut map = HashMap::<(i32, i32), char>::new();
    for chunk in &placed_chunks {
        for tile in &chunk.tiles_world {
            let x = tile.x + offset_x;
            let y = tile.y + offset_y;
            let ch = match tile.kind {
                TileKind::Solid => '#',
                TileKind::PlayerSpawn => '@',
                TileKind::Goal => 'G',
            };
            map.insert((x, y), ch);
        }
    }

    for &(x, y) in map.keys() {
        assert!(
            (0..MAP_WIDTH).contains(&x) && (0..MAP_HEIGHT).contains(&y),
            "タイル座標 ({x}, {y}) が許可サイズ {MAP_WIDTH}x{MAP_HEIGHT} を超えています"
        );
    }

    println!("== Placed Chunks ==");
    for chunk in &placed_chunks {
        println!("- {}", chunk.id);
    }

    println!();
    print_ascii_map(&map);
}

/// 指定方向の出口を1つ拾う（最小実装：最初の一致を返す）
fn pick_exit_dir(p: &PlacedChunk, want: Dir) -> Option<((i32, i32), Dir)> {
    p.exits_world.iter().copied().find(|(_, d)| *d == want)
}

/// 既存の“出口（ワールド座標）”に、次チャンクの“entry（ローカル）”を合わせる
fn place_next(
    template: &ChunkTemplate,
    required_entry_dir: Dir,
    exit: ((i32, i32), Dir),
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
fn place_chunk(t: &ChunkTemplate, origin: (i32, i32)) -> PlacedChunk {
    let entry_world = (origin.0 + t.entry.x, origin.1 + t.entry.y);
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
        origin,
        size: t.size,
        entry_world,
        exits_world,
        tiles_world,
    }
}

/// マップを囲いなしで出力（存在するタイルのmin/maxを計算して描画）
fn print_ascii_map(map: &HashMap<(i32, i32), char>) {
    for y in (0..MAP_HEIGHT).rev() {
        for x in 0..MAP_WIDTH {
            let ch = map.get(&(x, y)).copied().unwrap_or('.');
            print!("{ch}");
        }
        println!();
    }
}

/* ------------------------- サンプルチャンク ------------------------- */

fn chunk_start_flat() -> ChunkTemplate {
    // 6x4 の小部屋。右と上に出口。地面とプレイヤー初期位置あり。
    ChunkTemplate {
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

fn chunk_flat_bridge() -> ChunkTemplate {
    // 緩やかな傾斜で高さが少し上がるシンプルな橋。
    ChunkTemplate {
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

fn chunk_gap_jump() -> ChunkTemplate {
    // 小さな穴と段差。真ん中をジャンプで越えるイメージ。
    ChunkTemplate {
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

fn chunk_plateau() -> ChunkTemplate {
    // 高さ2の台地が中央にあるチャンク。
    ChunkTemplate {
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

fn chunk_stairs_up_small() -> ChunkTemplate {
    // 6x4。左から入って右上へ出る階段チャンク。さらに上方向の出口も一つ。
    ChunkTemplate {
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

fn chunk_stairs_down_small() -> ChunkTemplate {
    // 左が高台になっている小さな下り階段。
    ChunkTemplate {
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

fn chunk_goal_platform() -> ChunkTemplate {
    // 6x4。左から入って中段にゴールがある足場。右にも出口。
    ChunkTemplate {
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
                x: 4,
                y: 2,
                kind: TileKind::Goal,
            });
            v
        },
    }
}

fn chunk_goal_lower() -> ChunkTemplate {
    // 6x4。低めの足場にゴールが配置されたバリエーション。
    ChunkTemplate {
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
                x: 4,
                y: 1,
                kind: TileKind::Goal,
            });
            v
        },
    }
}
