use std::collections::HashSet;

use bevy::prelude::*;
use rand::prelude::*;
use rand::rngs::StdRng;
use shroom_core::*;

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
    mut rng: ResMut<GrowthRng>,
) {
    let mut tip_targets: Vec<(Entity, IVec2, RegionId)> = Vec::new();
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

        let mut best_score = f32::NEG_INFINITY;
        let mut best_pos = None;
        let is_infiltrator = region_states
            .get(tip.region_id)
            .is_some_and(|r| r.specialization == Some(SpecializationType::Infiltrator));

        for (npos, nentity) in grid.neighbors(pos) {
            if let Ok((_, ntile)) = tiles.get(nentity) {
                if !ntile.terrain.is_passable() {
                    continue;
                }
                if ntile.occupant.is_rival() && !is_infiltrator {
                    continue;
                }
                let offset = (npos - pos).as_vec2();
                let score = direction.dot(offset) + ntile.nutrient_level * 0.5;
                if score > best_score {
                    best_score = score;
                    best_pos = Some(npos);
                }
            }
        }

        match best_pos {
            Some(target) => tip_targets.push((tip_entity, target, tip.region_id)),
            None => tips_to_despawn.push(tip_entity),
        }
    }

    let mut claimed: HashSet<IVec2> = HashSet::new();
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

    fn spawn_tile_at(app: &mut App, pos: IVec2, tile: Tile) -> Entity {
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
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                occupant: Occupant::Player(rid),
                nutrient_gradient: Vec2::new(1.0, 0.0),
                biomass: 1.0,
                ..default()
            },
        );
        spawn_tile_at(&mut app, IVec2::new(6, 5), Tile::default());
        spawn_tile_at(&mut app, IVec2::new(4, 5), Tile::default());
        spawn_tile_at(&mut app, IVec2::new(5, 6), Tile::default());
        spawn_tile_at(&mut app, IVec2::new(5, 4), Tile::default());

        app.world_mut().spawn((
            GridPos(IVec2::new(5, 5)),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));

        app.add_systems(Update, hyphal_tip_system);
        app.update();

        let tips: Vec<IVec2> = app
            .world_mut()
            .query::<(&GridPos, &HyphalTip)>()
            .iter(app.world())
            .map(|(gp, _)| gp.0)
            .collect();
        assert_eq!(tips.len(), 1);
        assert_eq!(tips[0], IVec2::new(6, 5));

        let grid = app.world().resource::<GridWorld>();
        let target = app
            .world()
            .get::<Tile>(grid.tiles[&IVec2::new(6, 5)])
            .expect("tile should exist");
        assert!(target.occupant.is_player());
    }

    #[test]
    fn tip_dies_when_no_passable_neighbors() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                occupant: Occupant::Player(rid),
                biomass: 1.0,
                ..default()
            },
        );
        for &dir in &[
            IVec2::new(6, 5),
            IVec2::new(4, 5),
            IVec2::new(5, 6),
            IVec2::new(5, 4),
        ] {
            spawn_tile_at(
                &mut app,
                dir,
                Tile {
                    terrain: TerrainType::Rock,
                    ..default()
                },
            );
        }

        app.world_mut().spawn((
            GridPos(IVec2::new(5, 5)),
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
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                occupant: Occupant::Player(rid),
                nutrient_gradient: Vec2::new(1.0, 0.0),
                biomass: 2.0,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(7, 5),
            Tile {
                occupant: Occupant::Player(rid),
                nutrient_gradient: Vec2::new(-1.0, 0.0),
                biomass: 2.0,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(6, 5),
            Tile {
                occupant: Occupant::Player(rid),
                biomass: 1.0,
                ..default()
            },
        );

        app.world_mut().spawn((
            GridPos(IVec2::new(5, 5)),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));
        app.world_mut().spawn((
            GridPos(IVec2::new(7, 5)),
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
            .get::<Tile>(grid.tiles[&IVec2::new(6, 5)])
            .expect("tile should exist");
        assert!(
            tile.biomass > 1.0,
            "anastomosis should increase biomass, got {}",
            tile.biomass
        );
    }
}
