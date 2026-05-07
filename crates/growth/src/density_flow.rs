use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use kingdom_core::{
    AUTONOMOUS_FLOW_WEIGHT, BIASED_FLOW_WEIGHT, BIOMASS_CAP, CLAIM_THRESHOLD, FLOW_NOISE,
    GRADIENT_FLOW_WEIGHT, GridPos, GridWorld, Hex, HexLayout, MAX_OUTFLOW_FRACTION,
    MIN_FLOW_DENSITY, RegionId, Tile, TileDiscovered, WATER_GROWTH_COST,
};
use rand::SeedableRng;
use rand::prelude::*;
use rand::rngs::StdRng;

#[derive(Resource)]
pub struct DensityFlowRng(pub StdRng);

impl Default for DensityFlowRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(13))
    }
}

#[derive(Default)]
struct TileDelta {
    biomass_in: f32,
    biomass_out: f32,
    water_used: f32,
    region_shares: HashMap<RegionId, f32>,
}

#[derive(Clone, Copy)]
struct FlowSnapshot {
    region_id: Option<RegionId>,
    biomass: f32,
    bias: Vec2,
    gradient: Vec2,
    moisture: f32,
    passable: bool,
}

pub fn density_flow_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    layout: Res<HexLayout>,
    mut rng: ResMut<DensityFlowRng>,
    mut discovered: MessageWriter<TileDiscovered>,
) {
    let snapshot = build_snapshot(&tiles);
    let deltas = compute_deltas(&snapshot, &grid, &layout, &mut rng.0);
    apply_deltas(&mut tiles, &deltas, &mut discovered);
}

fn build_snapshot(tiles: &Query<(&GridPos, &mut Tile)>) -> HashMap<Hex, FlowSnapshot> {
    tiles
        .iter()
        .map(|(gp, t)| {
            (
                gp.0,
                FlowSnapshot {
                    region_id: t.region_id,
                    biomass: t.biomass,
                    bias: t.priority_bias,
                    gradient: t.nutrient_gradient,
                    moisture: t.moisture,
                    passable: t.terrain.is_passable(),
                },
            )
        })
        .collect()
}

fn compute_deltas(
    snapshot: &HashMap<Hex, FlowSnapshot>,
    grid: &GridWorld,
    layout: &HexLayout,
    rng: &mut StdRng,
) -> HashMap<Hex, TileDelta> {
    let mut deltas: HashMap<Hex, TileDelta> = HashMap::new();

    // Sort so rng draws are deterministic across runs with the same seed.
    let mut keys: Vec<Hex> = snapshot.keys().copied().collect();
    keys.sort_unstable_by_key(|h| (h.x, h.y));

    for pos in keys {
        let snap = snapshot[&pos];
        if snap.biomass <= MIN_FLOW_DENSITY {
            continue;
        }
        let Some(rid) = snap.region_id else { continue };

        let candidates = score_candidates(pos, snap, rid, snapshot, grid, layout, rng);
        let total: f32 = candidates.iter().map(|&(_, w)| w).sum();
        if total <= 0.0 {
            continue;
        }

        let max_outflow =
            (snap.biomass * MAX_OUTFLOW_FRACTION).min(snap.moisture / WATER_GROWTH_COST);
        if max_outflow <= 0.0 {
            continue;
        }

        for (npos, weight) in candidates {
            let share = max_outflow * (weight / total);
            let entry = deltas.entry(npos).or_default();
            entry.biomass_in += share;
            *entry.region_shares.entry(rid).or_insert(0.0) += share;
            let source = deltas.entry(pos).or_default();
            source.biomass_out += share;
            source.water_used += share * WATER_GROWTH_COST;
        }
    }

    deltas
}

fn score_candidates(
    pos: Hex,
    snap: FlowSnapshot,
    rid: RegionId,
    snapshot: &HashMap<Hex, FlowSnapshot>,
    grid: &GridWorld,
    layout: &HexLayout,
    rng: &mut StdRng,
) -> Vec<(Hex, f32)> {
    let from_world = layout.hex_to_world_pos(pos);
    let mut candidates: Vec<(Hex, f32)> = Vec::new();
    for (npos, _) in grid.neighbors(pos) {
        let Some(&n_snap) = snapshot.get(&npos) else {
            continue;
        };
        if !n_snap.passable {
            continue;
        }
        if let Some(other) = n_snap.region_id
            && other != rid
        {
            continue;
        }
        let to_world = layout.hex_to_world_pos(npos);
        let dir = (to_world - from_world).normalize_or_zero();
        let bias_score = snap.bias.dot(dir).max(0.0);
        let gradient_score = snap.gradient.dot(dir).max(0.0);
        let mut weight = AUTONOMOUS_FLOW_WEIGHT
            + BIASED_FLOW_WEIGHT * bias_score
            + GRADIENT_FLOW_WEIGHT * gradient_score;
        let noise = (rng.random::<f32>() - 0.5) * FLOW_NOISE;
        weight *= 1.0 + noise;
        if weight > 0.0 {
            candidates.push((npos, weight));
        }
    }
    candidates
}

fn apply_deltas(
    tiles: &mut Query<(&GridPos, &mut Tile)>,
    deltas: &HashMap<Hex, TileDelta>,
    discovered: &mut MessageWriter<TileDiscovered>,
) {
    for (gpos, mut tile) in tiles.iter_mut() {
        let Some(delta) = deltas.get(&gpos.0) else {
            continue;
        };
        if delta.biomass_out > 0.0 {
            tile.biomass = (tile.biomass - delta.biomass_out).max(0.0);
        }
        if delta.biomass_in > 0.0 {
            let new_biomass = (tile.biomass + delta.biomass_in).min(BIOMASS_CAP);
            let was_unowned = tile.region_id.is_none();
            tile.biomass = new_biomass;
            if was_unowned
                && new_biomass >= CLAIM_THRESHOLD
                && let Some((&rid, _)) =
                    delta.region_shares.iter().max_by(|(rid_a, a), (rid_b, b)| {
                        a.partial_cmp(b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| rid_b.0.cmp(&rid_a.0))
                    })
            {
                tile.region_id = Some(rid);
                if !tile.discovered {
                    tile.discovered = true;
                    discovered.write(TileDiscovered {
                        pos: gpos.0,
                        contents: tile.contents,
                    });
                }
            }
        }
        if delta.water_used > 0.0 {
            tile.moisture = (tile.moisture - delta.water_used).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{RegionStates, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.insert_resource(create_hex_layout());
        app.insert_resource(DensityFlowRng(StdRng::seed_from_u64(42)));
        app.add_message::<TileDiscovered>();
        app.add_systems(Update, density_flow_system);
        app
    }

    fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let e = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);
        e
    }

    #[test]
    fn flow_follows_bias_direction() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let layout = create_hex_layout();

        let center = Hex::new(5, 5);
        let neighbors = center.all_neighbors();
        let target = neighbors[0];
        let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

        spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                moisture: 1.0,
                priority_bias: dir,
                ..default()
            },
        );
        for &n in &neighbors {
            spawn(
                &mut app,
                n,
                Tile {
                    moisture: 0.5,
                    ..default()
                },
            );
        }

        app.update();

        let grid = app.world().resource::<GridWorld>();
        let target_tile = app.world().get::<Tile>(grid.tiles[&target]).unwrap();
        let other_tile = app.world().get::<Tile>(grid.tiles[&neighbors[3]]).unwrap();

        assert!(
            target_tile.biomass > other_tile.biomass,
            "biased neighbor ({}) should outpace opposite neighbor ({})",
            target_tile.biomass,
            other_tile.biomass
        );
    }

    #[test]
    fn flow_consumes_source_water() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let center = Hex::new(0, 0);
        let neighbors = center.all_neighbors();
        let source_e = spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                moisture: 1.0,
                ..default()
            },
        );
        for &n in &neighbors {
            spawn(
                &mut app,
                n,
                Tile {
                    moisture: 0.0,
                    ..default()
                },
            );
        }
        app.update();
        let m = app.world().get::<Tile>(source_e).unwrap().moisture;
        assert!(m < 1.0, "source moisture should drop after growth: {m}");
    }

    #[test]
    fn empty_tile_claimed_when_biomass_crosses_threshold() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let layout = create_hex_layout();
        let center = Hex::new(0, 0);
        let target = center.all_neighbors()[0];
        let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

        spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: BIOMASS_CAP,
                moisture: 1.0,
                priority_bias: dir * 1.5,
                ..default()
            },
        );
        let target_e = spawn(
            &mut app,
            target,
            Tile {
                moisture: 0.5,
                ..default()
            },
        );

        // Run multiple ticks — single-tick flow may not cross threshold.
        for _ in 0..20 {
            app.update();
        }

        let tile = app.world().get::<Tile>(target_e).unwrap();
        assert!(
            tile.biomass >= CLAIM_THRESHOLD,
            "expected target tile to cross claim threshold within 20 ticks; biomass = {}",
            tile.biomass
        );
        assert_eq!(tile.region_id, Some(rid));
    }

    #[test]
    fn cross_region_tiles_not_entered() {
        let mut app = test_app();
        let rid_a = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let rid_b = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let center = Hex::new(0, 0);
        let neighbor = center.all_neighbors()[0];
        spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid_a),
                biomass: 2.0,
                moisture: 1.0,
                priority_bias: Vec2::splat(1.0),
                ..default()
            },
        );
        let n_e = spawn(
            &mut app,
            neighbor,
            Tile {
                region_id: Some(rid_b),
                biomass: 0.5,
                moisture: 0.5,
                ..default()
            },
        );
        let initial = app.world().get::<Tile>(n_e).unwrap().biomass;
        for _ in 0..5 {
            app.update();
        }
        let after = app.world().get::<Tile>(n_e).unwrap().biomass;
        assert!(
            (after - initial).abs() < 1e-3,
            "biomass should be unchanged on a cross-region tile, was {initial} now {after}"
        );
    }
}
