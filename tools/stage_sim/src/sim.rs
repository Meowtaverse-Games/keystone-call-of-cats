use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{Result, anyhow, bail};

use crate::map::{GeneratedMap, StoneType, TileKind};

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

#[derive(Clone, Debug, Default, serde::Serialize)]
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
    stone_type: StoneType,
    dig_remaining: Vec<Option<u32>>,
    place_remaining: Option<u32>,
    pub metrics: Metrics,
}

#[derive(Clone)]
struct Snapshot {
    terrain: HashMap<Position, TileKind>,
    player: Position,
    stones: Vec<Position>,
    dig_remaining: Vec<Option<u32>>,
    place_remaining: Option<u32>,
}

impl World {
    pub fn from_map(map: &GeneratedMap, place_limit: Option<u32>) -> Result<Self> {
        let place_limit = place_limit.or(map.place_limit);
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
        let dig_remaining = vec![map.dig_limit; stones.len()];
        terrain.retain(|_, kind| !matches!(kind, TileKind::Player | TileKind::Stone));
        let initial = Snapshot {
            terrain: terrain.clone(),
            player,
            stones: stones.clone(),
            dig_remaining: dig_remaining.clone(),
            place_remaining: place_limit,
        };
        Ok(Self {
            size: map.canvas_size,
            terrain,
            initial,
            player,
            stones,
            stone_type: map.stone_type,
            dig_remaining,
            place_remaining: place_limit,
            metrics: Metrics::default(),
        })
    }

    pub fn reset(&mut self) {
        self.terrain = self.initial.terrain.clone();
        self.player = self.initial.player;
        self.stones = self.initial.stones.clone();
        self.dig_remaining = self.initial.dig_remaining.clone();
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
            stone_limit_text(&self.dig_remaining),
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
        let start = self.player;
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
        self.goal_route().is_some()
    }

    /// A replayable player-only path. Stones and obstacles remain stationary.
    /// Use exactly the same transitions as the manual player commands.
    pub fn goal_route(&self) -> Option<Vec<String>> {
        let mut parents = HashMap::new();
        let mut seen = HashSet::from([self.player]);
        let mut queue = VecDeque::from([self.player]);
        while let Some(position) = queue.pop_front() {
            if self.touches_goal(position) {
                let mut route = Vec::new();
                let mut cursor = position;
                while let Some(&(previous, action)) = parents.get(&cursor) {
                    route.push(format!("p {action}"));
                    cursor = previous;
                }
                route.reverse();
                return Some(route);
            }
            for action in PLAYER_ACTIONS {
                if let Some(next) = self.player_destination(position, action)
                    && seen.insert(next)
                {
                    parents.insert(next, (position, action));
                    queue.push_back(next);
                }
            }
        }
        None
    }

    pub fn walk_to_goal(&mut self) -> Result<Vec<String>> {
        let route = self.goal_route().ok_or_else(|| {
            anyhow!("no player-only route to goal with the current stones and terrain")
        })?;
        for action in &route {
            self.execute(action)?;
        }
        if !self.goal_reached() {
            bail!("computed player route did not reach the goal");
        }
        Ok(route)
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
            ["route"] => self
                .goal_route()
                .map(|route| route.join("\n"))
                .ok_or_else(|| anyhow!("no player-only route to goal")),
            ["walk-goal"] => {
                let route = self.walk_to_goal()?;
                Ok(format!("walked {} actions: {}", route.len(), self.status()))
            }
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
            ["assert", "unreachable"] => {
                if !self.goal_is_reachable() {
                    Ok("assert unreachable: ok (static grid)".to_owned())
                } else {
                    bail!("assert unreachable failed: player-only goal route exists")
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
                if (*action == "dig" && !matches!(self.stone_type, StoneType::Type3))
                    || (*action == "place" && !matches!(self.stone_type, StoneType::Type4))
                {
                    bail!("{:?} cannot {action}", self.stone_type);
                }
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
        let action = match direction {
            Direction::Left => "left",
            Direction::Right => "right",
            _ => bail!("player walk only supports left or right"),
        };
        self.apply_player_action(action)
    }

    fn player_jump(&mut self, dx: i32) -> Result<String> {
        self.apply_player_action(match dx {
            -1 => "jump-left",
            1 => "jump-right",
            _ => "jump",
        })
    }

    fn player_leap(&mut self, sign: i32) -> Result<String> {
        self.apply_player_action(if sign < 0 { "leap-left" } else { "leap-right" })
    }

    fn apply_player_action(&mut self, action: &str) -> Result<String> {
        self.metrics.actions += 1;
        self.metrics.player_actions += 1;
        if let Some(target) = self.player_destination(self.player, action) {
            self.player = target;
            Ok(format!("player -> {:?}", self.player))
        } else {
            self.metrics.blocked_actions += 1;
            Ok(format!("player {action} blocked"))
        }
    }

    fn player_destination(&self, position: Position, action: &str) -> Option<Position> {
        let (x, y) = position;
        let path = match action {
            "left" => vec![(x - 1, y)],
            "right" => vec![(x + 1, y)],
            "jump" => vec![(x, y + 1)],
            "jump-left" => vec![(x, y + 1), (x - 1, y + 1)],
            "jump-right" => vec![(x, y + 1), (x + 1, y + 1)],
            "leap-left" => vec![(x, y + 1), (x - 1, y + 2), (x - 2, y + 1)],
            "leap-right" => vec![(x, y + 1), (x + 1, y + 2), (x + 2, y + 1)],
            _ => return None,
        };
        if path.iter().all(|p| self.is_open_for_player(*p)) {
            self.settle(*path.last()?)
        } else {
            None
        }
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
        let Some(dig_remaining) = self.dig_remaining.get_mut(index) else {
            unreachable!("stone index was validated before its dig budget was read");
        };
        if *dig_remaining == Some(0) {
            self.metrics.blocked_actions += 1;
            return Ok("dig blocked: no uses remaining".to_owned());
        }
        if let Some(value) = dig_remaining {
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
        if !self.is_inside(target)
            || self.terrain.contains_key(&target)
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
        PLAYER_ACTIONS
            .iter()
            .filter_map(|action| self.player_destination(position, action))
            .collect()
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

fn stone_limit_text(limits: &[Option<u32>]) -> String {
    limits
        .iter()
        .enumerate()
        .map(|(index, limit)| format!("{index}:{}", limit_text(*limit)))
        .collect::<Vec<_>>()
        .join(",")
}

const PLAYER_ACTIONS: [&str; 7] = [
    "left",
    "right",
    "jump",
    "jump-left",
    "jump-right",
    "leap-left",
    "leap-right",
];

pub const HELP: &str = r#"commands:
  show
  status
  reachable
  reachmap
  route       (print player-only path; does not move anything)
  walk-goal   (replay that path; stones remain stationary)
  p left | p right
  p jump | p jump-left | p jump-right
  p leap-left | p leap-right
  s <index> move <left|right|up|down>
  s <index> dig <left|right|up|down>
  s <index> place <left|right|up|down>
  assert goal | assert reachable | assert unreachable
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
        world.stone_type = StoneType::Type4;
        assert!(world.execute("s 0 place up").unwrap().contains("placed"));
        assert!(world.execute("s 0 place left").unwrap().contains("no uses"));
    }

    #[test]
    fn stones_keep_separate_dig_budgets() {
        let generated = generate(
            &parse_stage(include_str!("../tests/fixtures/two_stones_scan_order.ron")).unwrap(),
            0,
        )
        .unwrap();
        let mut world = World::from_map(&generated, None).unwrap();

        for _ in 0..5 {
            assert!(!world.execute("s 0 dig down").unwrap().contains("no uses"));
        }
        assert!(!world.execute("s 1 dig down").unwrap().contains("no uses"));
        assert!(world.status().contains("dig=0:0,1:4"));
    }

    fn candidate() -> World {
        let config = parse_stage(include_str!("../examples/ai-candidate.ron")).unwrap();
        World::from_map(&generate(&config, 42).unwrap(), None).unwrap()
    }

    #[test]
    fn generated_route_is_replayable_and_reset_restores_the_unsolved_puzzle() {
        let mut world = candidate();
        assert!(world.goal_route().is_none());
        world.execute("s 0 place up").unwrap();
        let route = world.goal_route().unwrap();
        assert!(!route.is_empty());
        assert!(
            !world.goal_reached(),
            "search itself must not move the player"
        );
        for action in &route {
            world.execute(action).unwrap();
        }
        assert!(world.goal_reached());
        assert_eq!(world.metrics.blocked_actions, 0);
        assert_eq!(world.place_remaining, Some(0));
        world.reset();
        assert!(!world.goal_reached());
        assert!(world.goal_route().is_none());
        assert_eq!(world.place_remaining, Some(1));
    }

    #[test]
    fn failed_auto_walk_does_not_modify_the_world() {
        let mut world = candidate();
        let before = world.render(true);
        assert!(world.walk_to_goal().is_err());
        assert_eq!(world.render(true), before);
        assert_eq!(world.metrics.actions, 0);
    }

    #[test]
    fn manual_commands_obey_stone_type_and_ron_budget() {
        let mut world = candidate();
        assert!(world.execute("s 0 dig down").is_err());
        assert_eq!(world.metrics.actions, 0);
        assert!(world.execute("s 0 place down").unwrap().contains("placed"));
        assert!(world.execute("s 0 place up").unwrap().contains("no uses"));
        assert!(world.goal_route().is_none());
        world.stone_type = StoneType::Type1;
        assert!(world.execute("s 0 place up").is_err());
    }

    #[test]
    fn placing_on_occupied_tile_preserves_shared_budget() {
        let mut world = candidate();
        assert!(world.execute("s 0 place left").unwrap().contains("blocked"));
        assert_eq!(world.place_remaining, Some(1));
        world.execute("s 0 place up").unwrap();
        assert_eq!(world.place_remaining, Some(0));
    }
}
