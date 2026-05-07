use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexx::Hex;
use kingdom_core::{GridPos, GridWorld, HexLayout, RegionId, SelectedRegion, Tile};

/// Biomass drift below this threshold does not trigger a network mesh rebuild.
/// Density flow updates biomass continuously by ~0.01-0.5 per tick; rebuilding
/// the entire branch tree every tick costs hundreds of Mesh+Material asset
/// inserts. Real topology changes (tiles claimed/lost) bypass this via the
/// length and key checks.
const NETWORK_REBUILD_BIOMASS_TOLERANCE: f32 = 0.05;

#[derive(Resource, Default, Debug)]
pub struct BranchGraph {
    pub nodes: HashMap<Hex, BranchNode>,
    pub edges: Vec<BranchEdge>,
}

#[derive(Debug)]
pub struct BranchNode {
    pub pos: Hex,
    pub biomass: f32,
    pub region_id: RegionId,
}

#[derive(Debug)]
pub struct BranchEdge {
    pub from: Hex,
    pub to: Hex,
    pub thickness: f32,
}

#[derive(Resource, Default, Debug)]
pub struct RegionHulls {
    pub hulls: HashMap<RegionId, Vec<Vec2>>,
}

#[derive(Resource, Default, Debug)]
pub struct DiscoveryMap {
    /// 0.0 fully hidden, 1.0 fully revealed.
    pub discovered: HashMap<Hex, f32>,
}

#[derive(Resource, Default, Debug)]
pub struct SelectedRegionTiles {
    pub tiles: Vec<Hex>,
}

#[derive(Resource, Default, Debug)]
pub struct SelectedRegionExtractionRuns(pub u64);

pub fn extract_branch_graph(
    tiles: Query<(&GridPos, &Tile)>,
    grid: Res<GridWorld>,
    mut graph: ResMut<BranchGraph>,
    mut node_keys: Local<Vec<Hex>>,
    mut seen_edges: Local<HashSet<(Hex, Hex)>>,
) {
    let mut new_nodes: HashMap<Hex, BranchNode> = HashMap::with_capacity(graph.nodes.len());
    for (gpos, tile) in tiles.iter() {
        if tile.is_owned()
            && let Some(rid) = tile.region_id
        {
            new_nodes.insert(
                gpos.0,
                BranchNode {
                    pos: gpos.0,
                    biomass: tile.biomass,
                    region_id: rid,
                },
            );
        }
    }

    node_keys.clear();
    node_keys.extend(new_nodes.keys().copied());
    node_keys.sort_unstable_by_key(|h| (h.x, h.y));

    seen_edges.clear();
    let mut new_edges: Vec<BranchEdge> = Vec::with_capacity(graph.edges.len());
    for &pos in node_keys.iter() {
        for (npos, _) in grid.neighbors(pos) {
            if new_nodes.contains_key(&npos) {
                let edge_key = if (pos.x, pos.y) < (npos.x, npos.y) {
                    (pos, npos)
                } else {
                    (npos, pos)
                };
                if seen_edges.insert(edge_key) {
                    let from_biomass = new_nodes[&pos].biomass;
                    let to_biomass = new_nodes[&npos].biomass;
                    new_edges.push(BranchEdge {
                        from: pos,
                        to: npos,
                        thickness: (from_biomass + to_biomass) * 0.5,
                    });
                }
            }
        }
    }

    if new_nodes.len() != graph.nodes.len()
        || new_edges.len() != graph.edges.len()
        || !nodes_match(&new_nodes, &graph.nodes)
        || !edges_match(&new_edges, &graph.edges)
    {
        graph.nodes = new_nodes;
        graph.edges = new_edges;
    }
}

fn nodes_match(a: &HashMap<Hex, BranchNode>, b: &HashMap<Hex, BranchNode>) -> bool {
    a.iter().all(|(k, v)| {
        b.get(k).is_some_and(|other| {
            other.region_id == v.region_id
                && (other.biomass - v.biomass).abs() < NETWORK_REBUILD_BIOMASS_TOLERANCE
        })
    })
}

fn edges_match(a: &[BranchEdge], b: &[BranchEdge]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| {
        x.from == y.from
            && x.to == y.to
            && (x.thickness - y.thickness).abs() < NETWORK_REBUILD_BIOMASS_TOLERANCE
    })
}

pub fn extract_region_hulls(
    tiles: Query<(&GridPos, &Tile)>,
    layout: Res<HexLayout>,
    mut hulls: ResMut<RegionHulls>,
    mut region_positions: Local<HashMap<RegionId, Vec<Vec2>>>,
    mut new_hulls: Local<HashMap<RegionId, Vec<Vec2>>>,
) {
    for v in region_positions.values_mut() {
        v.clear();
    }
    for (gpos, tile) in tiles.iter() {
        if let Some(rid) = tile.region_id {
            region_positions
                .entry(rid)
                .or_default()
                .push(layout.hex_to_world_pos(gpos.0));
        }
    }

    new_hulls.clear();
    for (rid, positions) in region_positions.iter() {
        if positions.is_empty() {
            continue;
        }
        if positions.len() < 2 {
            new_hulls.insert(*rid, positions.clone());
            continue;
        }
        let min_x = positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = positions
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        new_hulls.insert(
            *rid,
            vec![
                Vec2::new(min_x - 0.5, min_y - 0.5),
                Vec2::new(max_x + 0.5, min_y - 0.5),
                Vec2::new(max_x + 0.5, max_y + 0.5),
                Vec2::new(min_x - 0.5, max_y + 0.5),
            ],
        );
    }

    region_positions.retain(|_, v| !v.is_empty());

    if !hulls_match(&new_hulls, &hulls.hulls) {
        hulls.hulls.clone_from(&new_hulls);
    }
}

fn hulls_match(a: &HashMap<RegionId, Vec<Vec2>>, b: &HashMap<RegionId, Vec<Vec2>>) -> bool {
    a.len() == b.len() && a.iter().all(|(k, v)| b.get(k) == Some(v))
}

pub fn extract_discovery_map(
    graph: Res<BranchGraph>,
    mut discovery: ResMut<DiscoveryMap>,
    mut influence_map: Local<HashMap<Hex, f32>>,
    mut new_discovered: Local<HashMap<Hex, f32>>,
) {
    if !graph.is_changed() {
        return;
    }

    const RADIUS: u32 = 8;
    const FULLY_HIDDEN: f32 = 0.02;
    const FULLY_VISIBLE: f32 = 0.12;

    influence_map.clear();
    for &node_pos in graph.nodes.keys() {
        influence_map.insert(node_pos, f32::MAX);
    }

    for &node_pos in graph.nodes.keys() {
        for tile in node_pos.range(RADIUS) {
            if graph.nodes.contains_key(&tile) {
                continue;
            }
            let hex_dist = tile.unsigned_distance_to(node_pos) as f32;
            let noise = (tile.x.wrapping_mul(73_856_093) ^ tile.y.wrapping_mul(19_349_663)) as f32
                / (i32::MAX as f32);
            if hex_dist + noise * 1.5 > RADIUS as f32 {
                continue;
            }
            let influence = 1.0 / (hex_dist * hex_dist + 1.0);
            *influence_map.entry(tile).or_default() += influence;
        }
    }

    new_discovered.clear();
    for (tile, influence) in influence_map.iter() {
        let discovered = if *influence <= FULLY_HIDDEN {
            0.0
        } else if *influence >= FULLY_VISIBLE {
            1.0
        } else {
            (*influence - FULLY_HIDDEN) / (FULLY_VISIBLE - FULLY_HIDDEN)
        };
        if discovered > 0.0 {
            new_discovered.insert(*tile, discovered);
        }
    }

    if discovery.discovered != *new_discovered {
        discovery.discovered.clone_from(&new_discovered);
    }
}

pub fn extract_selected_region_tiles(
    tiles: Query<(&GridPos, &Tile)>,
    changed: Query<(), Changed<Tile>>,
    selected: Res<SelectedRegion>,
    mut selected_tiles: ResMut<SelectedRegionTiles>,
    mut runs: ResMut<SelectedRegionExtractionRuns>,
) {
    if !selected.is_changed() && changed.is_empty() {
        return;
    }
    runs.0 += 1;
    let new_tiles: Vec<Hex> = match selected.region_id {
        Some(rid) => tiles
            .iter()
            .filter_map(|(gpos, tile)| (tile.region_id == Some(rid)).then_some(gpos.0))
            .collect(),
        None => Vec::new(),
    };
    if selected_tiles.tiles != new_tiles {
        selected_tiles.tiles = new_tiles;
    }
}

#[cfg(test)]
mod tests {
    use kingdom_core::{RegionStates, create_hex_layout};

    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<BranchGraph>();
        app.init_resource::<RegionHulls>();
        app.init_resource::<SelectedRegionExtractionRuns>();
        app.insert_resource(create_hex_layout());
        app
    }

    #[test]
    fn branch_graph_extracts_player_network() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        for q in 0..3 {
            let pos = Hex::new(q, 0);
            let e = app
                .world_mut()
                .spawn((
                    GridPos(pos),
                    Tile {
                        region_id: Some(rid),
                        biomass: 1.0,
                        ..default()
                    },
                ))
                .id();
            app.world_mut()
                .resource_mut::<GridWorld>()
                .tiles
                .insert(pos, e);
        }

        app.add_systems(Update, extract_branch_graph);
        app.update();

        let graph = app.world().resource::<BranchGraph>();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn region_hulls_produces_bounding_box() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        for q in 0..3 {
            for r in 0..2 {
                let pos = Hex::new(q, r);
                app.world_mut().spawn((
                    GridPos(pos),
                    Tile {
                        region_id: Some(rid),
                        ..default()
                    },
                ));
            }
        }

        app.add_systems(Update, extract_region_hulls);
        app.update();

        let hulls = app.world().resource::<RegionHulls>();
        assert!(hulls.hulls.contains_key(&rid));
        let hull = &hulls.hulls[&rid];
        assert_eq!(hull.len(), 4); // bounding box has 4 corners
    }

    #[test]
    fn discovery_map_inverse_square_gradient() {
        let mut app = test_app();
        app.init_resource::<DiscoveryMap>();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let pos = Hex::new(10, 10);
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);

        app.add_systems(
            Update,
            (extract_branch_graph, extract_discovery_map).chain(),
        );
        app.update();

        let discovery = app.world().resource::<DiscoveryMap>();

        // Center tile: fully discovered
        let center = discovery
            .discovered
            .get(&Hex::new(10, 10))
            .copied()
            .unwrap_or(0.0);
        assert_eq!(center, 1.0);

        // Distance 2: well-discovered with lowered thresholds
        let near = discovery
            .discovered
            .get(&Hex::new(12, 10))
            .copied()
            .unwrap_or(0.0);
        assert!(near > 0.5, "near tile should be well-discovered: {near}");

        // Distance 7: barely discovered (near edge of radius 8)
        let far = discovery
            .discovered
            .get(&Hex::new(17, 10))
            .copied()
            .unwrap_or(0.0);
        assert!(far < 0.5, "far tile should be dimly discovered: {far}");

        // Distance 9: outside radius 8
        assert!(
            !discovery.discovered.contains_key(&Hex::new(19, 10)),
            "tiles outside radius 8 should not be in the map"
        );
    }

    #[test]
    fn extract_branch_graph_does_not_run_outside_simulation_set() {
        use kingdom_core::SimulationSystems;

        let mut app = test_app();
        app.configure_sets(Update, SimulationSystems.run_if(|| false));
        app.add_systems(Update, extract_branch_graph.in_set(SimulationSystems));

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = Hex::ZERO;
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);

        app.update();

        let graph = app.world().resource::<BranchGraph>();
        assert!(graph.nodes.is_empty(), "system ran despite gate");
    }

    #[test]
    fn small_biomass_drift_does_not_rebuild_graph() {
        use kingdom_core::RegionStates;

        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = Hex::new(0, 0);
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);

        app.add_systems(Update, extract_branch_graph);
        app.update();

        let first_tick = app.world().resource_ref::<BranchGraph>().last_changed();

        app.world_mut().get_mut::<Tile>(e).unwrap().biomass = 1.0 + 0.01;
        app.update();

        let second_tick = app.world().resource_ref::<BranchGraph>().last_changed();
        assert_eq!(
            first_tick, second_tick,
            "sub-tolerance biomass drift must not flag BranchGraph as changed"
        );
    }

    #[test]
    fn large_biomass_change_rebuilds_graph() {
        use kingdom_core::RegionStates;

        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = Hex::new(0, 0);
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);

        app.add_systems(Update, extract_branch_graph);
        app.update();
        let first_tick = app.world().resource_ref::<BranchGraph>().last_changed();

        app.world_mut().get_mut::<Tile>(e).unwrap().biomass = 5.0;
        app.update();

        let second_tick = app.world().resource_ref::<BranchGraph>().last_changed();
        assert_ne!(
            first_tick, second_tick,
            "supra-tolerance biomass change must flag BranchGraph as changed"
        );
    }

    #[test]
    fn selected_region_extraction_skips_when_unchanged() {
        use kingdom_core::SelectedRegion;

        let mut app = test_app();
        app.init_resource::<SelectedRegion>();
        app.init_resource::<SelectedRegionTiles>();
        app.add_systems(Update, extract_selected_region_tiles);

        let rid = app
            .world_mut()
            .resource_mut::<kingdom_core::RegionStates>()
            .create_region();
        let pos = Hex::new(2, 2);
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: Some(rid),
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);
        app.world_mut().resource_mut::<SelectedRegion>().region_id = Some(rid);

        app.update();
        let runs_after_frame_1 = app.world().resource::<SelectedRegionExtractionRuns>().0;
        assert_eq!(
            runs_after_frame_1, 1,
            "body should run once when SelectedRegion changed"
        );

        app.update();
        let runs_after_frame_2 = app.world().resource::<SelectedRegionExtractionRuns>().0;
        assert_eq!(
            runs_after_frame_2, 1,
            "body must not run again when no input changed"
        );

        app.world_mut().get_mut::<Tile>(e).unwrap().biomass = 0.5;
        app.update();
        let runs_after_frame_3 = app.world().resource::<SelectedRegionExtractionRuns>().0;
        assert_eq!(
            runs_after_frame_3, 2,
            "body must run again when any Tile changed"
        );
    }
}
