use std::collections::HashSet;

use bevy::prelude::*;
use fungai_core::*;
use rand::prelude::*;
use rand::rngs::StdRng;

#[derive(Resource)]
pub struct GrowthRng(pub StdRng);

impl Default for GrowthRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(0))
    }
}

pub fn hyphal_tip_system(
    mut commands: Commands,
    mut tips: Query<(Entity, &GridPos, &mut HyphalTip)>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
    layout: Res<HexLayout>,
    mut rng: ResMut<GrowthRng>,
) {
    let mut tip_targets: Vec<(Entity, Hex, RegionId)> = Vec::new();
    let mut tips_to_despawn: Vec<Entity> = Vec::new();

    for (tip_entity, gpos, mut tip) in tips.iter_mut() {
        tip.age += 1;
        let pos = gpos.0;

        let (gradient, bias, region_nutrients) = {
            let tile = grid
                .tiles
                .get(&pos)
                .and_then(|&e| tiles.get(e).ok())
                .map(|(_, t)| t);
            let grad = tile.map_or(Vec2::ZERO, |t| t.nutrient_gradient);
            let bias = tile.map_or(Vec2::ZERO, |t| t.priority_bias);
            let nutrients = region_states
                .get(tip.region_id)
                .map_or(0.0, |r| r.nutrients);
            (grad, bias, nutrients)
        };

        if region_nutrients < 0.1 {
            tips_to_despawn.push(tip_entity);
            continue;
        }

        let combined = gradient + bias;
        let jitter = Vec2::new(
            rng.0.random::<f32>() * 0.4 - 0.2,
            rng.0.random::<f32>() * 0.4 - 0.2,
        );
        let direction = combined + jitter;

        let is_infiltrator = region_states
            .get(tip.region_id)
            .is_some_and(|r| r.specialization == Some(SpecializationType::Infiltrator));

        // Score all passable neighbors, separating frontier (empty/rival) from owned
        let mut best_frontier_score = f32::NEG_INFINITY;
        let mut best_frontier_pos = None;

        for (npos, nentity) in grid.neighbors(pos) {
            if let Ok((_, ntile)) = tiles.get(nentity) {
                if !ntile.terrain.is_passable() {
                    continue;
                }
                if ntile.occupant.is_player() {
                    continue;
                }
                if ntile.occupant.is_rival() && !is_infiltrator {
                    continue;
                }
                let from_world = layout.hex_to_world_pos(pos);
                let to_world = layout.hex_to_world_pos(npos);
                let offset = (to_world - from_world).normalize_or_zero();
                let score = direction.dot(offset) + ntile.nutrient_level * 0.5;
                if score > best_frontier_score {
                    best_frontier_score = score;
                    best_frontier_pos = Some(npos);
                }
            }
        }

        match best_frontier_pos {
            Some(target) => tip_targets.push((tip_entity, target, tip.region_id)),
            None => tips_to_despawn.push(tip_entity),
        }
    }

    let mut claimed: HashSet<Hex> = HashSet::new();
    for (tip_entity, target, rid) in &tip_targets {
        if claimed.contains(target) {
            if let Some(&tentity) = grid.tiles.get(target)
                && let Ok((_, mut tile)) = tiles.get_mut(tentity)
            {
                tile.biomass += tile.biomass * ANASTOMOSIS_BIOMASS_BONUS;
            }
            commands.entity(*tip_entity).despawn();
            continue;
        }
        claimed.insert(*target);

        if let Some(&tentity) = grid.tiles.get(target)
            && let Ok((_, mut tile)) = tiles.get_mut(tentity)
        {
            if tile.occupant == Occupant::Empty {
                tile.occupant = Occupant::Player(*rid);
                tile.biomass = 0.5;
            } else if tile.occupant.is_rival() {
                // Infiltrator flips rival tile at 50% biomass
                tile.occupant = Occupant::Player(*rid);
                tile.biomass *= 0.5;
            }
        }

        commands.entity(*tip_entity).insert(GridPos(*target));
    }

    for entity in tips_to_despawn {
        commands.entity(entity).despawn();
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
        app.insert_resource(GrowthRng(StdRng::seed_from_u64(42)));
        app
    }

    fn spawn_tile_at(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let entity = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);
        entity
    }

    #[test]
    fn tip_moves_toward_nutrient_gradient() {
        let mut app = test_app();
        let layout = create_hex_layout();
        app.insert_resource(layout);

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let center = Hex::new(5, 5);
        let neighbors = center.all_neighbors();
        // Pick a target neighbor and compute the world-space direction for the gradient
        let target = neighbors[0];
        let layout = app.world().resource::<HexLayout>();
        let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

        spawn_tile_at(
            &mut app,
            center,
            Tile {
                occupant: Occupant::Player(rid),
                nutrient_gradient: dir,
                biomass: 1.0,
                ..default()
            },
        );
        for &n in &neighbors {
            spawn_tile_at(&mut app, n, Tile::default());
        }

        app.world_mut().spawn((
            GridPos(center),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));

        app.add_systems(Update, hyphal_tip_system);
        app.update();

        let tips: Vec<Hex> = app
            .world_mut()
            .query::<(&GridPos, &HyphalTip)>()
            .iter(app.world())
            .map(|(gp, _)| gp.0)
            .collect();
        assert_eq!(tips.len(), 1);
        assert_eq!(tips[0], target);

        let grid = app.world().resource::<GridWorld>();
        let target_tile = app
            .world()
            .get::<Tile>(grid.tiles[&target])
            .expect("tile should exist");
        assert!(target_tile.occupant.is_player());
    }

    #[test]
    fn tip_dies_when_no_passable_neighbors() {
        let mut app = test_app();
        app.insert_resource(create_hex_layout());

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let center = Hex::new(5, 5);
        spawn_tile_at(
            &mut app,
            center,
            Tile {
                occupant: Occupant::Player(rid),
                biomass: 1.0,
                ..default()
            },
        );
        for n in center.all_neighbors() {
            spawn_tile_at(
                &mut app,
                n,
                Tile {
                    terrain: TerrainType::Rock,
                    ..default()
                },
            );
        }

        app.world_mut().spawn((
            GridPos(center),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));

        app.add_systems(Update, hyphal_tip_system);
        app.update();

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(
            tip_count, 0,
            "tip should despawn when surrounded by impassable terrain"
        );
    }

    #[test]
    fn tip_anastomosis_boosts_biomass() {
        let mut app = test_app();
        let layout = create_hex_layout();
        app.insert_resource(layout);

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        // Two tips on opposite sides of a shared neighbor, both pointing toward it
        let shared = Hex::new(5, 5);
        let shared_neighbors = shared.all_neighbors();
        let tip_a = shared_neighbors[0];
        let tip_b = shared_neighbors[3]; // opposite side

        let layout = app.world().resource::<HexLayout>();
        let dir_a = (layout.hex_to_world_pos(shared) - layout.hex_to_world_pos(tip_a)).normalize();
        let dir_b = (layout.hex_to_world_pos(shared) - layout.hex_to_world_pos(tip_b)).normalize();

        spawn_tile_at(
            &mut app,
            tip_a,
            Tile {
                occupant: Occupant::Player(rid),
                nutrient_gradient: dir_a,
                biomass: 2.0,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            tip_b,
            Tile {
                occupant: Occupant::Player(rid),
                nutrient_gradient: dir_b,
                biomass: 2.0,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            shared,
            Tile {
                biomass: 1.0,
                ..default()
            },
        );

        app.world_mut().spawn((
            GridPos(tip_a),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));
        app.world_mut().spawn((
            GridPos(tip_b),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));

        app.add_systems(Update, hyphal_tip_system);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let tile = app
            .world()
            .get::<Tile>(grid.tiles[&shared])
            .expect("tile should exist");
        assert!(
            tile.biomass > 0.5,
            "anastomosis should increase biomass beyond initial claim, got {}",
            tile.biomass
        );
    }
}
