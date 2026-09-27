use std::collections::HashSet;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    resources::chunk_grammar_map::TileKind,
    scenes::stage::components::{StageRoot, StageTile, StoneIndex, StoneRune},
    util::script_types::MoveDirection,
};

use super::StonePlaceRequestMessage;

#[derive(Component)]
pub struct PlacedTile;

#[derive(Resource, Clone)]
pub struct StageGridMetrics {
    pub local_tile_size: Vec2,
    pub viewport_size: Vec2,
    pub sprite_scale: f32,
    pub map_size: (isize, isize),
    pub boundary_margin: (isize, isize),
}

impl StageGridMetrics {
    pub fn cell_to_local(&self, cell: IVec2) -> Vec2 {
        Vec2::new(
            (cell.x as f32 + 0.5) * self.local_tile_size.x - self.viewport_size.x * 0.5,
            (cell.y as f32 + 0.5) * self.local_tile_size.y - self.viewport_size.y * 0.5,
        )
    }

    fn world_to_cell(&self, world: Vec2, root: &GlobalTransform) -> IVec2 {
        let root_scale = root.scale().truncate();
        let local = (world - root.translation().truncate()) / root_scale;
        IVec2::new(
            ((local.x + self.viewport_size.x * 0.5) / self.local_tile_size.x).floor() as i32,
            ((local.y + self.viewport_size.y * 0.5) / self.local_tile_size.y).floor() as i32,
        )
    }

    fn world_tile_size(&self, root: &GlobalTransform) -> Vec2 {
        self.local_tile_size * root.scale().truncate()
    }

    fn is_placeable_cell(&self, cell: IVec2) -> bool {
        let x = cell.x as isize;
        let y = cell.y as isize;
        x >= self.boundary_margin.0
            && x < self.map_size.0 - self.boundary_margin.0
            && y >= self.boundary_margin.1
            && y < self.map_size.1 - self.boundary_margin.1
    }
}

#[derive(Resource, Default)]
pub struct PlaceState {
    initial_limit: Option<u32>,
    pub remaining: Option<u32>,
}

impl PlaceState {
    pub fn new(limit: Option<u32>) -> Self {
        Self {
            initial_limit: limit,
            remaining: limit,
        }
    }

    fn can_commit(&self) -> bool {
        self.remaining != Some(0)
    }

    fn commit(&mut self) {
        if let Some(remaining) = &mut self.remaining {
            *remaining = remaining.saturating_sub(1);
        }
    }

    fn reset(&mut self) {
        self.remaining = self.initial_limit;
    }
}

fn direction_to_vec(direction: MoveDirection) -> Vec2 {
    match direction {
        MoveDirection::Left => Vec2::NEG_X,
        MoveDirection::Top => Vec2::Y,
        MoveDirection::Right => Vec2::X,
        MoveDirection::Down => Vec2::NEG_Y,
    }
}

pub fn resolve_place_requests(
    mut commands: Commands,
    mut requests: MessageReader<StonePlaceRequestMessage>,
    metrics: Option<Res<StageGridMetrics>>,
    mut state: ResMut<PlaceState>,
    roots: Query<(Entity, &GlobalTransform), With<StageRoot>>,
    stones: Query<(&StoneIndex, &GlobalTransform), With<StoneRune>>,
    spatial: SpatialQuery,
) {
    let Some(metrics) = metrics else {
        return;
    };
    let Some((stage_root, root_transform)) = roots.iter().next() else {
        return;
    };

    let mut requests: Vec<_> = requests.read().cloned().collect();
    requests.sort_by_key(|request| request.stone_index);
    let mut reserved_cells = HashSet::new();

    for request in requests {
        let Ok((_, stone_transform)) = stones.get(request.stone) else {
            continue;
        };
        let target = stone_transform.translation().truncate()
            + direction_to_vec(request.direction) * metrics.world_tile_size(root_transform);
        let cell = metrics.world_to_cell(target, root_transform);
        let local = metrics.cell_to_local(cell);
        let target_world =
            root_transform.translation().truncate() + local * root_transform.scale().truncate();
        if !metrics.is_placeable_cell(cell) || !state.can_commit() || !reserved_cells.insert(cell) {
            continue;
        }

        let candidate = Collider::rectangle(
            metrics.world_tile_size(root_transform).x * 0.998,
            metrics.world_tile_size(root_transform).y * 0.998,
        );
        let filter = SpatialQueryFilter::default().with_excluded_entities([request.stone]);
        if !spatial
            .shape_intersections(&candidate, target_world, 0.0, &filter)
            .is_empty()
        {
            reserved_cells.remove(&cell);
            continue;
        }

        commands.entity(stage_root).with_children(|parent| {
            parent.spawn((
                PlacedTile,
                StageTile,
                TileKind::Solid,
                Sprite {
                    color: Color::srgb(0.25, 0.65, 0.92),
                    custom_size: Some(Vec2::splat(16.0)),
                    ..default()
                },
                Transform::from_xyz(local.x, local.y, -4.0)
                    .with_scale(Vec3::splat(metrics.sprite_scale)),
                RigidBody::Static,
                Collider::rectangle(16.0, 16.0),
            ));
        });
        state.commit();
    }
}

pub fn reset_placed_tiles(
    mut commands: Commands,
    editor: Res<super::ui::ScriptEditorState>,
    mut state: ResMut<PlaceState>,
    placed_tiles: Query<Entity, With<PlacedTile>>,
) {
    if !editor.pending_player_reset {
        return;
    }
    for entity in placed_tiles.iter() {
        commands.entity(entity).try_despawn();
    }
    state.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metrics() -> StageGridMetrics {
        StageGridMetrics {
            local_tile_size: Vec2::ONE,
            viewport_size: Vec2::ZERO,
            sprite_scale: 1.0,
            map_size: (8, 8),
            boundary_margin: (1, 1),
        }
    }

    fn placement_app(limit: Option<u32>) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, PhysicsPlugins::default()))
            .insert_resource(test_metrics())
            .insert_resource(PlaceState::new(limit))
            .add_message::<StonePlaceRequestMessage>()
            .add_systems(Update, resolve_place_requests);
        app
    }

    fn spawn_stage_and_stone(app: &mut App, index: usize, cell: IVec2) -> (Entity, Entity) {
        let root = app
            .world_mut()
            .spawn((StageRoot, Transform::default(), GlobalTransform::default()))
            .id();
        let position = test_metrics().cell_to_local(cell);
        let stone = app
            .world_mut()
            .spawn((
                StoneRune,
                StoneIndex(index),
                Transform::from_translation(position.extend(0.0)),
                GlobalTransform::from(Transform::from_translation(position.extend(0.0))),
            ))
            .id();
        (root, stone)
    }

    fn request_place(app: &mut App, stone: Entity, stone_index: usize) {
        app.world_mut()
            .resource_mut::<Messages<StonePlaceRequestMessage>>()
            .write(StonePlaceRequestMessage {
                stone,
                stone_index,
                direction: MoveDirection::Right,
            });
    }

    fn placed_tiles(app: &mut App) -> Vec<Entity> {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<PlacedTile>>();
        query.iter(world).collect()
    }

    #[test]
    fn place_limit_consumes_only_successful_commits_and_resets() {
        let mut state = PlaceState::new(Some(1));
        let cell = IVec2::new(3, 4);
        assert!(state.can_commit());
        state.commit();
        assert_eq!(state.remaining, Some(0));
        assert!(!state.can_commit());
        state.reset();
        assert_eq!(state.remaining, Some(1));
        assert!(state.can_commit());
    }

    #[test]
    fn grid_rejects_boundaries_and_accepts_interior_cells() {
        let metrics = test_metrics();
        assert!(!metrics.is_placeable_cell(IVec2::new(0, 4)));
        assert!(!metrics.is_placeable_cell(IVec2::new(9, 4)));
        assert!(!metrics.is_placeable_cell(IVec2::new(4, 0)));
        assert!(metrics.is_placeable_cell(IVec2::new(4, 4)));
    }

    #[test]
    fn every_place_direction_targets_the_adjacent_cell() {
        assert_eq!(direction_to_vec(MoveDirection::Left), Vec2::NEG_X);
        assert_eq!(direction_to_vec(MoveDirection::Top), Vec2::Y);
        assert_eq!(direction_to_vec(MoveDirection::Right), Vec2::X);
        assert_eq!(direction_to_vec(MoveDirection::Down), Vec2::NEG_Y);
    }

    #[test]
    fn reset_despawns_placed_tiles_and_restores_the_shared_limit() {
        let mut app = App::new();
        app.insert_resource(PlaceState {
            initial_limit: Some(2),
            remaining: Some(0),
        })
        .insert_resource(super::super::ui::ScriptEditorState {
            pending_player_reset: true,
            ..default()
        })
        .add_systems(Update, reset_placed_tiles);

        let tile = app.world_mut().spawn(PlacedTile).id();
        app.update();

        assert!(app.world().get_entity(tile).is_err());
        let state = app.world().resource::<PlaceState>();
        assert_eq!(state.remaining, Some(2));
    }

    #[test]
    fn resolver_spawns_one_stage_child_with_a_static_collider_and_charges_once() {
        let mut app = placement_app(Some(1));
        let (root, stone) = spawn_stage_and_stone(&mut app, 0, IVec2::new(2, 2));
        app.update();
        request_place(&mut app, stone, 0);
        app.update();

        let placed = placed_tiles(&mut app);
        assert_eq!(placed.len(), 1);
        let block = placed[0];
        assert!(app.world().get::<Collider>(block).is_some());
        assert_eq!(
            app.world().get::<RigidBody>(block),
            Some(&RigidBody::Static)
        );
        assert_eq!(
            app.world().get::<ChildOf>(block).map(ChildOf::parent),
            Some(root)
        );
        assert_eq!(app.world().resource::<PlaceState>().remaining, Some(0));
    }

    #[test]
    fn resolver_rejects_live_collider_without_charging_the_shared_limit() {
        let mut app = placement_app(Some(1));
        let (_, stone) = spawn_stage_and_stone(&mut app, 0, IVec2::new(2, 2));
        let target = test_metrics().cell_to_local(IVec2::new(3, 2));
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::rectangle(1.0, 1.0),
            Transform::from_translation(target.extend(0.0)),
            GlobalTransform::from(Transform::from_translation(target.extend(0.0))),
        ));
        app.update();
        request_place(&mut app, stone, 0);
        app.update();

        assert!(placed_tiles(&mut app).is_empty());
        assert_eq!(app.world().resource::<PlaceState>().remaining, Some(1));
    }

    #[test]
    fn resolver_resolves_same_cell_conflicts_by_stone_index() {
        let mut app = placement_app(Some(1));
        let (_, first) = spawn_stage_and_stone(&mut app, 0, IVec2::new(2, 2));
        let (_, second) = spawn_stage_and_stone(&mut app, 1, IVec2::new(2, 2));
        app.update();
        request_place(&mut app, second, 1);
        request_place(&mut app, first, 0);
        app.update();

        assert_eq!(placed_tiles(&mut app).len(), 1);
        assert_eq!(app.world().resource::<PlaceState>().remaining, Some(0));
    }
}
