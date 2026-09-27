use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{Result, anyhow, bail};

use crate::map::{GeneratedMap, TileKind};

type Position = (i32, i32);

#[derive(Clone, Copy, Debug)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn parse(input: &str) -> Result<Self> {
        match input.to_ascii_lowercase().as_str() {
            "left" | "l" => Ok(Self::Left),
            "right" | "r" => Ok(Self::Right),
            "up" | "top" | "u" => Ok(Self::Up),
            "down" | "d" => Ok(Self::Down),
            _ => bail!("unknown direction '{input}'"),
        }
    }

    fn delta(self) -> Position {
        match self {
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
            Self::Up => (0, 1),
            Self::Down => (0, -1),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub actions: usize,
    pub player_actions: usize,
    pub stone_moves: usize,
    pub digs: usize,
    pub places: usize,
    pub blocked_actions: usize,
}

#[derive(Clone)]
pub struct World {
    size: (i32, i32),
    terrain: HashMap<Position, TileKind>,
    initial: Snapshot,
    player: Position,
    stones: Vec<Position>,
    dig_remaining: Option<u32>,
    place_remaining: Option<u32>,
    pub metrics: Metrics,
}

#[derive(Clone)]
struct Snapshot {
    terrain: HashMap<Position, TileKind>,
    player: Position,
    stones: Vec<Position>,
    dig_remaining: Option<u32>,
    place_remaining: Option<u32>,
}

impl World {
    pub fn from_map(map: &GeneratedMap, place_limit: Option<u32>) -> Result<Self> {
        let mut terrain = map.tiles.clone();
        let players = map.positions(TileKind::Player);
        if players.len() != 1 {
            bail!(
                "stage must have exactly one player spawn, found {}",
                players.len()
            );
        }
        let player = players[0];
        let stones = map.positions(TileKind::Stone);
        terrain.retain(|_, kind| !matches!(kind, TileKind::Player | TileKind::Stone));
        let initial = Snapshot {
            terrain: terrain.clone(),
            player,
            stones: stones.clone(),
            dig_remaining: map.dig_limit,
            place_remaining: place_limit,
        };
        Ok(Self {
            size: map.canvas_size,
            terrain,
            initial,
            player,
            stones,
            dig_remaining: map.dig_limit,
            place_remaining: place_limit,
            metrics: Metrics::default(),
        })
    }

    pub fn reset(&mut self) {
        self.terrain = self.initial.terrain.clone();
        self.player = self.initial.player;
        self.stones = self.initial.stones.clone();
        self.dig_remaining = self.initial.dig_remaining;
        self.place_remaining = self.initial.place_remaining;
        self.metrics = Metrics::default();
    }

    pub fn render(&self, coordinates: bool) -> String {
        self.render_with_overlay(coordinates, None)
    }

    pub fn render_reachable(&self, coordinates: bool) -> String {
        let reachable = self.reachable_positions();
        self.render_with_overlay(coordinates, Some(&reachable))
    }

    fn render_with_overlay(
        &self,
        coordinates: bool,
        reachable: Option<&HashSet<Position>>,
    ) -> String {
        let mut output = String::new();
        if coordinates {
            output.push_str("    ");
            for x in 0..self.size.0 {
                output.push(char::from_digit((x / 10) as u32, 10).unwrap_or(' '));
            }
            output.push('\n');
            output.push_str("    ");
            for x in 0..self.size.0 {
                output.push(char::from_digit((x % 10) as u32, 10).unwrap_or('?'));
            }
            output.push('\n');
        }
        for y in (0..self.size.1).rev() {
            if coordinates {
                output.push_str(&format!("{y:>3} "));
            }
            for x in 0..self.size.0 {
                let position = (x, y);
                let character = if self.player == position {
                    '@'
                } else if let Some(index) = self.stones.iter().position(|stone| *stone == position)
                {
                    if self.stones.len() == 1 {
                        'S'
                    } else {
                        char::from_digit(index as u32, 10).unwrap_or('S')
                    }
                } else {
                    self.terrain
                        .get(&position)
                        .copied()
                        .map(TileKind::symbol)
                        .unwrap_or_else(|| {
                            if reachable.is_some_and(|cells| cells.contains(&position)) {
                                ':'
                            } else {
                                '.'
                            }
                        })
                };
                output.push(character);
            }
            output.push('\n');
        }
        output
    }

    pub fn status(&self) -> String {
        format!(
            "player={:?} stones={:?} dig={} place={} goal={} actions={} blocked={}",
            self.player,
            self.stones,
            limit_text(self.dig_remaining),
            limit_text(self.place_remaining),
            if self.goal_reached() {
                "reached"
            } else {
                "not-reached"
            },
            self.metrics.actions,
            self.metrics.blocked_actions,
        )
    }

    pub fn goal_reached(&self) -> bool {
        self.touches_goal(self.player)
    }

    pub fn reachable_positions(&self) -> HashSet<Position> {
        let start = self.settle(self.player).unwrap_or(self.player);
        let mut reached = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(position) = queue.pop_front() {
            for next in self.player_neighbors(position) {
                if reached.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        reached
    }

    pub fn goal_is_reachable(&self) -> bool {
        self.reachable_positions()
            .into_iter()
            .any(|position| self.touches_goal(position))
    }

    pub fn program_is_empty(&self, stone_index: usize, delta: (i32, i32)) -> bool {
        let Some(source) = self.stones.get(stone_index).copied() else {
            return false;
        };
        let target = (source.0 + delta.0, source.1 + delta.1);
        !self.is_blocked(target)
            && !self
                .stones
                .iter()
                .enumerate()
                .any(|(index, stone)| index != stone_index && *stone == target)
    }

    pub fn program_is_touched(&self, stone_index: usize) -> bool {
        let Some(stone) = self.stones.get(stone_index).copied() else {
            return false;
        };
        (self.player.0 - stone.0).abs() + (self.player.1 - stone.1).abs() <= 1
    }

    pub fn execute(&mut self, input: &str) -> Result<String> {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(String::new());
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["show"] => Ok(self.render(true)),
            ["status"] => Ok(self.status()),
            ["reachable"] => {
                let count = self.reachable_positions().len();
                Ok(format!(
                    "reachable-cells={count} goal-reachable={}",
                    self.goal_is_reachable()
                ))
            }
            ["reachmap"] => Ok(self.render_reachable(true)),
            ["reset"] => {
                self.reset();
                Ok("reset".to_owned())
            }
            ["assert", "goal"] => {
                if self.goal_reached() {
                    Ok("assert goal: ok".to_owned())
                } else {
                    bail!("assert goal failed")
                }
            }
            ["assert", "reachable"] => {
                if self.goal_is_reachable() {
                    Ok("assert reachable: ok".to_owned())
                } else {
                    bail!("assert reachable failed")
                }
            }
            ["p", "left"] | ["player", "left"] => self.player_walk(Direction::Left),
            ["p", "right"] | ["player", "right"] => self.player_walk(Direction::Right),
            ["p", "jump"] | ["player", "jump"] => self.player_jump(0),
            ["p", "jump-left"] | ["player", "jump-left"] => self.player_jump(-1),
            ["p", "jump-right"] | ["player", "jump-right"] => self.player_jump(1),
            ["p", "leap-left"] | ["player", "leap-left"] => self.player_leap(-1),
            ["p", "leap-right"] | ["player", "leap-right"] => self.player_leap(1),
            ["s", index, action, direction] | ["stone", index, action, direction] => {
                let index = index
                    .parse::<usize>()
                    .map_err(|_| anyhow!("invalid stone index '{index}'"))?;
                let direction = Direction::parse(direction)?;
                match *action {
                    "move" => self.stone_move(index, direction),
                    "dig" => self.stone_dig(index, direction),
                    "place" => self.stone_place(index, direction),
                    _ => bail!("unknown stone action '{action}'"),
                }
            }
            _ => bail!("unknown command '{trimmed}' (use 'help' in play mode)"),
        }
    }

    fn player_walk(&mut self, direction: Direction) -> Result<String> {
        let dx = match direction {
            Direction::Left => -1,
            Direction::Right => 1,
            _ => bail!("player walk only supports left or right"),
        };
        self.metrics.actions += 1;
        self.metrics.player_actions += 1;
        let target = (self.player.0 + dx, self.player.1);
        if !self.is_open_for_player(target) {
            self.metrics.blocked_actions += 1;
            return Ok(format!("player walk blocked at {target:?}"));
        }
        self.player = self.settle(target).unwrap_or(target);
        Ok(format!("player -> {:?}", self.player))
    }

    fn player_jump(&mut self, dx: i32) -> Result<String> {
        self.metrics.actions += 1;
        self.metrics.player_actions += 1;
        let above = (self.player.0, self.player.1 + 1);
        let target = (self.player.0 + dx, self.player.1 + 1);
        if !self.is_open_for_player(above) || !self.is_open_for_player(target) {
            self.metrics.blocked_actions += 1;
            return Ok(format!("player jump blocked at {target:?}"));
        }
        self.player = self.settle(target).unwrap_or(target);
        Ok(format!("player -> {:?}", self.player))
    }

    fn player_leap(&mut self, sign: i32) -> Result<String> {
        self.metrics.actions += 1;
        self.metrics.player_actions += 1;
        let path = [
            (self.player.0, self.player.1 + 1),
            (self.player.0 + sign, self.player.1 + 2),
            (self.player.0 + sign * 2, self.player.1 + 1),
        ];
        if path
            .iter()
            .any(|position| !self.is_open_for_player(*position))
        {
            self.metrics.blocked_actions += 1;
            return Ok("player leap blocked".to_owned());
        }
        self.player = self.settle(path[2]).unwrap_or(path[2]);
        Ok(format!("player -> {:?}", self.player))
    }

    fn stone_move(&mut self, index: usize, direction: Direction) -> Result<String> {
        let source = self.stone(index)?;
        self.metrics.actions += 1;
        self.metrics.stone_moves += 1;
        let delta = direction.delta();
        let target = (source.0 + delta.0, source.1 + delta.1);
        if self.is_blocked(target) || self.stones.contains(&target) {
            self.metrics.blocked_actions += 1;
            return Ok(format!("stone {index} move blocked at {target:?}"));
        }

        let rider = self.player == (source.0, source.1 + 1);
        if rider {
            let player_target = (self.player.0 + delta.0, self.player.1 + delta.1);
            if self.is_blocked(player_target)
                || self
                    .stones
                    .iter()
                    .enumerate()
                    .any(|(other, stone)| other != index && *stone == player_target)
            {
                self.metrics.blocked_actions += 1;
                return Ok(format!(
                    "stone {index} cannot carry player to {player_target:?}"
                ));
            }
            self.player = player_target;
        }
        self.stones[index] = target;
        Ok(format!("stone {index} -> {target:?}"))
    }

    fn stone_dig(&mut self, index: usize, direction: Direction) -> Result<String> {
        let source = self.stone(index)?;
        self.metrics.actions += 1;
        self.metrics.digs += 1;
        if self.dig_remaining == Some(0) {
            self.metrics.blocked_actions += 1;
            return Ok("dig blocked: no uses remaining".to_owned());
        }
        if let Some(value) = &mut self.dig_remaining {
            *value = value.saturating_sub(1);
        }
        let delta = direction.delta();
        let target = (source.0 + delta.0, source.1 + delta.1);
        match self.terrain.get(&target).copied() {
            Some(
                TileKind::Solid | TileKind::Obstacle | TileKind::DynamicSolid | TileKind::Placed,
            ) => {
                self.terrain.remove(&target);
                Ok(format!("dug tile at {target:?}"))
            }
            Some(TileKind::Wall | TileKind::Goal) | None => {
                self.metrics.blocked_actions += 1;
                Ok(format!("dig had no effect at {target:?}"))
            }
            Some(TileKind::Player | TileKind::Stone) => unreachable!(),
        }
    }

    fn stone_place(&mut self, index: usize, direction: Direction) -> Result<String> {
        let source = self.stone(index)?;
        self.metrics.actions += 1;
        self.metrics.places += 1;
        if self.place_remaining == Some(0) {
            self.metrics.blocked_actions += 1;
            return Ok("place blocked: no uses remaining".to_owned());
        }
        let delta = direction.delta();
        let target = (source.0 + delta.0, source.1 + delta.1);
        if self.terrain.contains_key(&target)
            || self.player == target
            || self.stones.contains(&target)
        {
            self.metrics.blocked_actions += 1;
            return Ok(format!("place blocked at {target:?}"));
        }
        if let Some(value) = &mut self.place_remaining {
            *value = value.saturating_sub(1);
        }
        self.terrain.insert(target, TileKind::Placed);
        Ok(format!("placed tile at {target:?}"))
    }

    fn stone(&self, index: usize) -> Result<Position> {
        self.stones
            .get(index)
            .copied()
            .ok_or_else(|| anyhow!("stone {index} does not exist"))
    }

    fn is_inside(&self, position: Position) -> bool {
        position.0 >= 0 && position.0 < self.size.0 && position.1 >= 0 && position.1 < self.size.1
    }

    fn is_blocked(&self, position: Position) -> bool {
        !self.is_inside(position)
            || self
                .terrain
                .get(&position)
                .copied()
                .is_some_and(TileKind::blocks)
    }

    fn is_open_for_player(&self, position: Position) -> bool {
        !self.is_blocked(position) && !self.stones.contains(&position)
    }

    fn has_support(&self, position: Position) -> bool {
        let below = (position.0, position.1 - 1);
        self.is_blocked(below) || self.stones.contains(&below)
    }

    fn settle(&self, mut position: Position) -> Option<Position> {
        if !self.is_open_for_player(position) {
            return None;
        }
        while position.1 > 0 && !self.has_support(position) {
            let next = (position.0, position.1 - 1);
            if !self.is_open_for_player(next) {
                break;
            }
            position = next;
        }
        Some(position)
    }

    fn player_neighbors(&self, position: Position) -> Vec<Position> {
        let mut neighbors = Vec::new();
        for dx in [-1, 1] {
            let candidate = (position.0 + dx, position.1);
            if let Some(landing) = self.settle(candidate)
                && landing != position
            {
                neighbors.push(landing);
            }
        }

        let headroom = (position.0, position.1 + 1);
        if self.is_open_for_player(headroom) {
            for dx in -1..=1 {
                let candidate = (position.0 + dx, position.1 + 1);
                if self.is_open_for_player(candidate)
                    && let Some(landing) = self.settle(candidate)
                {
                    neighbors.push(landing);
                }
            }

            for sign in [-1, 1] {
                let path = [
                    headroom,
                    (position.0 + sign, position.1 + 2),
                    (position.0 + sign * 2, position.1 + 1),
                ];
                if path
                    .iter()
                    .all(|candidate| self.is_open_for_player(*candidate))
                    && let Some(landing) = self.settle(path[2])
                {
                    neighbors.push(landing);
                }
            }
        }
        neighbors.sort_unstable();
        neighbors.dedup();
        neighbors
    }

    fn touches_goal(&self, position: Position) -> bool {
        [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)]
            .into_iter()
            .map(|delta| (position.0 + delta.0, position.1 + delta.1))
            .any(|candidate| self.terrain.get(&candidate) == Some(&TileKind::Goal))
    }
}

fn limit_text(value: Option<u32>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "unlimited".to_owned())
}

pub const HELP: &str = r#"commands:
  show
  status
  reachable
  reachmap
  p left | p right
  p jump | p jump-left | p jump-right
  p leap-left | p leap-right
  s <index> move <left|right|up|down>
  s <index> dig <left|right|up|down>
  s <index> place <left|right|up|down>
  assert goal | assert reachable
  reset
  help
  quit
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{generate, parse_stage};

    fn world(dig_limit: u32, place_limit: u32) -> World {
        let input = format!(
            r#####"(
                map_size: (8, 6),
                stone_type: Type3,
                dig_limit: Some({dig_limit}),
                start_chunks: [ChunkTemplate(id: "start", map: ["@S#E", "####"])],
                middle_chunks: [ChunkTemplate(id: "noop", map: ["IE"])],
                goal_chunks: [ChunkTemplate(id: "goal", map: ["IG"])],
            )"#####
        );
        let generated = generate(&parse_stage(&input).unwrap(), 1).unwrap();
        World::from_map(&generated, Some(place_limit)).unwrap()
    }

    #[test]
    fn stone_can_dig_and_move_into_tile() {
        let mut world = world(1, 0);
        assert!(world.execute("s 0 move right").unwrap().contains("blocked"));
        assert!(world.execute("s 0 dig right").unwrap().contains("dug"));
        assert!(world.execute("s 0 move right").unwrap().contains("stone 0"));
    }

    #[test]
    fn placed_tile_uses_budget() {
        let mut world = world(0, 1);
        assert!(world.execute("s 0 place up").unwrap().contains("placed"));
        assert!(world.execute("s 0 place left").unwrap().contains("no uses"));
    }
}
