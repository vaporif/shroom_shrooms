use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use shroom_core::{GridPos, GridWorld, Occupant, RegionId, RegionStates, Tile};

pub fn region_tracking_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    let mut player_tiles: HashMap<IVec2, RegionId> = HashMap::default();
    for (gpos, tile) in tiles.iter() {
        if let Occupant::Player(rid) = tile.occupant {
            player_tiles.insert(gpos.0, rid);
        }
    }

    for state in region_states.regions.values_mut() {
        state.tile_count = 0;
    }

    let mut visited: HashSet<IVec2> = HashSet::default();
    let mut components: Vec<(RegionId, Vec<IVec2>)> = Vec::new();

    for (&pos, &original_rid) in &player_tiles {
        if visited.contains(&pos) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![pos];
        while let Some(p) = stack.pop() {
            if !visited.insert(p) {
                continue;
            }
            if player_tiles.contains_key(&p) {
                component.push(p);
                for (neighbor, _) in grid.neighbors(p) {
                    if !visited.contains(&neighbor) && player_tiles.contains_key(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
        if !component.is_empty() {
            components.push((original_rid, component));
        }
    }

    // First connected component keeps the original id, splits get new ones
    let mut seen_rids: HashSet<RegionId> = HashSet::default();
    for (original_rid, positions) in &components {
        let rid = if seen_rids.insert(*original_rid) {
            *original_rid
        } else {
            region_states.create_region()
        };

        let biomass_sum: f32 = positions
            .iter()
            .filter_map(|p| grid.tiles.get(p))
            .filter_map(|&e| tiles.get(e).ok())
            .map(|(_, t)| t.biomass)
            .sum();

        if let Some(state) = region_states.get_mut(rid) {
            state.tile_count = positions.len() as u32;
            state.biomass = biomass_sum;
        }

        for &pos in positions {
            if let Some(&entity) = grid.tiles.get(&pos)
                && let Ok((_, mut tile)) = tiles.get_mut(entity)
            {
                tile.occupant = Occupant::Player(rid);
            }
        }
    }

    let empty_rids: Vec<RegionId> = region_states
        .regions
        .iter()
        .filter(|(_, s)| s.tile_count == 0)
        .map(|(id, _)| *id)
        .collect();
    for rid in empty_rids {
        region_states.remove(rid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app
    }

    fn spawn_tile(app: &mut App, pos: IVec2, occupant: Occupant) -> Entity {
        let entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);
        entity
    }

    #[test]
    fn connected_player_tiles_form_one_region() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        spawn_tile(&mut app, IVec2::new(0, 0), Occupant::Player(rid));
        spawn_tile(&mut app, IVec2::new(1, 0), Occupant::Player(rid));
        spawn_tile(&mut app, IVec2::new(2, 0), Occupant::Player(rid));

        app.add_systems(Update, region_tracking_system);
        app.update();

        let regions = app.world().resource::<RegionStates>();
        let state = regions.get(rid).unwrap();
        assert_eq!(state.tile_count, 3);
    }

    #[test]
    fn disconnected_tiles_split_into_two_regions() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        spawn_tile(&mut app, IVec2::new(0, 0), Occupant::Player(rid));
        spawn_tile(&mut app, IVec2::new(1, 0), Occupant::Player(rid));
        spawn_tile(&mut app, IVec2::new(2, 0), Occupant::Empty); // gap
        spawn_tile(&mut app, IVec2::new(3, 0), Occupant::Player(rid));
        spawn_tile(&mut app, IVec2::new(4, 0), Occupant::Player(rid));

        app.add_systems(Update, region_tracking_system);
        app.update();

        let regions = app.world().resource::<RegionStates>();
        let total_player_tiles: u32 = regions.regions.values().map(|r| r.tile_count).sum();
        assert_eq!(total_player_tiles, 4);
        assert!(regions.regions.len() >= 2);
    }
}
