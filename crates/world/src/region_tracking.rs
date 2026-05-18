use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexx::Hex;
use kingdom_core::{GridPos, GridWorld, RegionId, RegionStates, Tile, Unit};

pub fn region_tracking_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut units: Query<&mut Unit>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    // Pass 1 — read-only snapshot of owned tiles and their biomass.
    let mut owned: HashMap<Hex, RegionId> = HashMap::default();
    let mut biomass: HashMap<Hex, f32> = HashMap::default();
    for (gpos, tile) in tiles.iter() {
        if tile.is_owned()
            && let Some(rid) = tile.region_id
        {
            owned.insert(gpos.0, rid);
            biomass.insert(gpos.0, tile.biomass);
        }
    }

    for state in region_states.regions.values_mut() {
        state.tile_count = 0;
        state.total_biomass = 0.0;
    }

    // Sort components deterministically before processing: split pieces call
    // create_region(), so the iteration order fixes the new region ids. Sorting
    // by each component's minimum hex makes that order reproducible and
    // independent of HashMap iteration order.
    let mut components = connected_components(&owned, &grid);
    components.sort_by_cached_key(|(_, hexes)| {
        hexes
            .iter()
            .map(|h| (h.x, h.y))
            .min()
            .expect("component is never empty")
    });

    // An id is "absorbed" if it appears as a non-minimum member of any
    // component — a merge swallows it there, so it must not survive anywhere.
    // Every other piece still carrying that id becomes a fresh region instead
    // of keeping it. Computing this set up front (rather than deciding per
    // component) is what makes a simultaneous merge-and-split of the same
    // region deterministic instead of corrupting its id.
    let mut absorbed: HashSet<RegionId> = HashSet::default();
    for (member_ids, _) in &components {
        let candidate = member_ids
            .iter()
            .copied()
            .min()
            .expect("a component always carries at least one member id");
        for &rid in member_ids {
            if rid != candidate {
                absorbed.insert(rid);
            }
        }
    }

    // Pass 2 — assign each component its survivor id, in the sorted order. The
    // first component to want an un-absorbed `candidate` keeps it. A later
    // component with the same candidate is a split; so is any component whose
    // candidate is absorbed elsewhere. Both get a fresh region with an empty
    // bank — RegionState::new's default is a don't-care, the empty economy
    // comes from this explicit assignment.
    let mut claimed: HashSet<RegionId> = HashSet::default();
    let mut survivors: Vec<RegionId> = Vec::with_capacity(components.len());
    for (member_ids, _) in &components {
        let candidate = member_ids
            .iter()
            .copied()
            .min()
            .expect("a component always carries at least one member id");
        let survivor = if !absorbed.contains(&candidate) && claimed.insert(candidate) {
            candidate
        } else {
            let fresh = region_states.create_region();
            if let Some(state) = region_states.get_mut(fresh) {
                state.sugars = 0.0;
                state.melanin = 0.0;
            }
            fresh
        };
        survivors.push(survivor);
    }

    // Pass 3 — fold each absorbed region's resources into the survivor of the
    // first component (in sorted order) that merges it, then remove it.
    // `reparent` doubles as the drain-once ledger here and is consumed by the
    // unit re-parenting pass added in Task 3.
    let mut reparent: HashMap<RegionId, RegionId> = HashMap::default();
    for ((member_ids, _), &survivor) in components.iter().zip(&survivors) {
        let candidate = member_ids
            .iter()
            .copied()
            .min()
            .expect("a component always carries at least one member id");
        for &rid in member_ids {
            if rid == candidate || reparent.contains_key(&rid) {
                continue;
            }
            if let Some(state) = region_states.remove(rid)
                && let Some(survivor_state) = region_states.get_mut(survivor)
            {
                survivor_state.sugars += state.sugars;
                survivor_state.melanin += state.melanin;
            }
            reparent.insert(rid, survivor);
        }
    }

    // Re-parent the units of every absorbed region onto its survivor, so a
    // founder produced by an absorbed network keeps paying upkeep.
    if !reparent.is_empty() {
        for mut unit in &mut units {
            if let Some(&survivor) = reparent.get(&unit.owner) {
                unit.owner = survivor;
            }
        }
    }

    // Pass 4 — relabel every tile to its component's survivor and tally it.
    for ((_, hexes), &survivor) in components.iter().zip(&survivors) {
        let biomass_sum: f32 = hexes.iter().filter_map(|h| biomass.get(h)).sum();
        if let Some(state) = region_states.get_mut(survivor) {
            state.tile_count = hexes.len() as u32;
            state.total_biomass = biomass_sum;
        }
        for &pos in hexes {
            if let Some(&entity) = grid.tiles.get(&pos)
                && let Ok((_, mut tile)) = tiles.get_mut(entity)
            {
                tile.region_id = Some(survivor);
            }
        }
    }

    region_states.regions.retain(|_, s| s.tile_count > 0);
}

fn connected_components(
    owned: &HashMap<Hex, RegionId>,
    grid: &GridWorld,
) -> Vec<(HashSet<RegionId>, Vec<Hex>)> {
    let mut visited: HashSet<Hex> = HashSet::default();
    let mut components = Vec::new();

    for &start in owned.keys() {
        if visited.contains(&start) {
            continue;
        }
        let mut component = Vec::new();
        let mut member_ids = HashSet::default();
        let mut stack = vec![start];
        while let Some(p) = stack.pop() {
            if !visited.insert(p) {
                continue;
            }
            component.push(p);
            if let Some(&rid) = owned.get(&p) {
                member_ids.insert(rid);
            }
            for (neighbor, _) in grid.neighbors(p) {
                if !visited.contains(&neighbor) && owned.contains_key(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        if !component.is_empty() {
            components.push((member_ids, component));
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
        app.add_systems(Update, region_tracking_system);
        app
    }

    fn spawn_tile(app: &mut App, pos: Hex, region_id: Option<RegionId>, biomass: f32) -> Entity {
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
    fn contiguous_regions_merge_to_lowest_id() {
        let mut app = test_app();
        let old = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let young = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        assert!(old.0 < young.0);
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(old)
            .unwrap()
            .sugars = 30.0;
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(young)
            .unwrap()
            .sugars = 12.0;

        // Three adjacent tiles bridge the two regions into one component.
        spawn_tile(&mut app, Hex::new(0, 0), Some(old), 1.0);
        spawn_tile(&mut app, Hex::new(1, 0), Some(young), 1.0);
        spawn_tile(&mut app, Hex::new(2, 0), Some(young), 1.0);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert!(rs.get(young).is_none(), "younger region should be absorbed");
        let survivor = rs.get(old).unwrap();
        assert_eq!(survivor.tile_count, 3);
        assert_eq!(survivor.sugars, 42.0, "absorbed sugars pool into survivor");
    }

    #[test]
    fn severed_split_piece_gets_a_fresh_empty_region() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(rid)
            .unwrap()
            .sugars = 50.0;

        // Two clusters, no connecting tile. They share `rid` as their min member id,
        // so one keeps it (the cluster sorting first by lowest hex) and the other splits.
        spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
        spawn_tile(&mut app, Hex::new(1, 0), Some(rid), 1.0);
        spawn_tile(&mut app, Hex::new(5, 0), Some(rid), 0.4);
        spawn_tile(&mut app, Hex::new(6, 0), Some(rid), 0.9);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert_eq!(rs.regions.len(), 2);
        let kept = rs.get(rid).unwrap();
        assert_eq!(
            kept.sugars, 50.0,
            "the id-keeping component keeps its resources"
        );
        let split = rs.regions.values().find(|s| s.region_id != rid).unwrap();
        assert_eq!(
            split.sugars, 0.0,
            "the split piece rebuilds its own economy"
        );
        assert_eq!(split.melanin, 0.0);
    }

    #[test]
    fn merge_and_split_of_same_region_in_one_tick() {
        let mut app = test_app();
        let a = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let b = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        assert!(a.0 < b.0, "A must be older so it survives the merge");
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(a)
            .unwrap()
            .sugars = 20.0;
        {
            let mut rs = app.world_mut().resource_mut::<RegionStates>();
            let bs = rs.get_mut(b).unwrap();
            bs.sugars = 15.0;
            bs.melanin = 7.0;
        }

        // Merge component: an A tile bridged to a B tile — B is a non-minimum
        // member here, so it lands in `absorbed`.
        spawn_tile(&mut app, Hex::new(0, 0), Some(a), 1.0);
        spawn_tile(&mut app, Hex::new(1, 0), Some(b), 1.0);
        // Severed cluster still tagged B, disconnected from the merge component.
        spawn_tile(&mut app, Hex::new(8, 0), Some(b), 1.0);
        spawn_tile(&mut app, Hex::new(9, 0), Some(b), 1.0);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert!(rs.get(b).is_none(), "B is absorbed and must not survive");
        let survivor = rs.get(a).unwrap();
        assert_eq!(survivor.tile_count, 2, "A keeps the merge component");
        assert_eq!(survivor.sugars, 35.0, "B's sugars fold into A exactly once");
        assert_eq!(
            survivor.melanin, 7.0,
            "B's melanin folds into A exactly once"
        );

        assert_eq!(rs.regions.len(), 2);
        let fresh = rs.regions.values().find(|s| s.region_id != a).unwrap();
        assert_eq!(fresh.tile_count, 2, "the severed cluster is its own region");
        assert_eq!(fresh.sugars, 0.0, "the severed cluster starts empty");
        assert_eq!(fresh.melanin, 0.0);
    }

    #[test]
    fn empty_region_is_removed() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 0.05);
        app.update();
        assert!(app.world().resource::<RegionStates>().get(rid).is_none());
    }

    #[test]
    fn merge_reparents_absorbed_units_to_the_survivor() {
        use kingdom_core::{Unit, UnitKind};

        let mut app = test_app();
        let old = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let young = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let unit = app
            .world_mut()
            .spawn((
                GridPos(Hex::new(9, 9)),
                Unit {
                    kind: UnitKind::Founder,
                    owner: young,
                },
            ))
            .id();

        spawn_tile(&mut app, Hex::new(0, 0), Some(old), 1.0);
        spawn_tile(&mut app, Hex::new(1, 0), Some(young), 1.0);
        app.update();

        assert!(app.world().resource::<RegionStates>().get(young).is_none());
        assert_eq!(
            app.world().get::<Unit>(unit).unwrap().owner,
            old,
            "the absorbed region's unit follows the merge survivor",
        );
    }
}
