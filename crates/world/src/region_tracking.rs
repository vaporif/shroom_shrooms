use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexx::Hex;
use kingdom_core::{GridPos, GridWorld, RegionId, RegionStates, Tile};

pub fn region_tracking_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    // THRESHOLD-GATED: a tile is "owned" only when its biomass has actually
    // reached CLAIM_THRESHOLD. Sub-threshold region_id tags (e.g. fresh density
    // flow that hasn't accumulated yet) do not contribute to region tracking.
    let player_tiles: HashMap<Hex, RegionId> = tiles
        .iter()
        .filter(|(_, tile)| tile.is_owned())
        .filter_map(|(gpos, tile)| tile.region_id.map(|rid| (gpos.0, rid)))
        .collect();

    for state in region_states.regions.values_mut() {
        state.tile_count = 0;
        state.total_biomass = 0.0;
    }

    let components = connected_components(&player_tiles, &grid);

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
            state.total_biomass = biomass_sum;
        }

        if rid != *original_rid {
            for &pos in positions {
                if let Some(&entity) = grid.tiles.get(&pos)
                    && let Ok((_, mut tile)) = tiles.get_mut(entity)
                {
                    tile.region_id = Some(rid);
                }
            }
        }
    }

    region_states.regions.retain(|_, s| s.tile_count > 0);
}

fn connected_components(
    player_tiles: &HashMap<Hex, RegionId>,
    grid: &GridWorld,
) -> Vec<(RegionId, Vec<Hex>)> {
    let mut visited: HashSet<Hex> = HashSet::default();
    let mut components = Vec::new();

    for (&start, &original_rid) in player_tiles {
        if visited.contains(&start) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(p) = stack.pop() {
            if !visited.insert(p) {
                continue;
            }
            component.push(p);
            for (neighbor, _) in grid.neighbors(p) {
                if !visited.contains(&neighbor) && player_tiles.contains_key(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        if !component.is_empty() {
            components.push((original_rid, component));
        }
    }
    components
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

    fn spawn_tile(app: &mut App, pos: Hex, region_id: Option<RegionId>) -> Entity {
        // For tracking tests, owned tiles need biomass >= CLAIM_THRESHOLD to
        // satisfy the THRESHOLD-GATED ownership semantics.
        let biomass = if region_id.is_some() { 0.5 } else { 0.0 };
        let entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id,
                    biomass,
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

        // Three horizontally adjacent hexes (axial coords: q varies, r=0)
        spawn_tile(&mut app, Hex::new(0, 0), Some(rid));
        spawn_tile(&mut app, Hex::new(1, 0), Some(rid));
        spawn_tile(&mut app, Hex::new(2, 0), Some(rid));

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

        // Two clusters separated by a gap (non-adjacent in hex space)
        spawn_tile(&mut app, Hex::new(0, 0), Some(rid));
        spawn_tile(&mut app, Hex::new(1, 0), Some(rid));
        spawn_tile(&mut app, Hex::new(2, 0), None);
        spawn_tile(&mut app, Hex::new(3, 0), Some(rid));
        spawn_tile(&mut app, Hex::new(4, 0), Some(rid));

        app.add_systems(Update, region_tracking_system);
        app.update();

        let regions = app.world().resource::<RegionStates>();
        let total_player_tiles: u32 = regions.regions.values().map(|r| r.tile_count).sum();
        assert_eq!(total_player_tiles, 4);
        assert!(regions.regions.len() >= 2);
    }
}
