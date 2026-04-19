use std::collections::HashMap;

use bevy::prelude::*;
use shroom_core::*;

pub fn nutrient_gradient_system(mut tiles: Query<(&GridPos, &mut Tile)>, grid: Res<GridWorld>) {
    let nutrient_map: HashMap<IVec2, f32> = tiles
        .iter()
        .map(|(gp, t)| (gp.0, t.nutrient_level))
        .collect();

    for (gpos, mut tile) in tiles.iter_mut() {
        let pos = gpos.0;
        let mut gradient = Vec2::ZERO;
        let my_nutrient = tile.nutrient_level;

        for (npos, _) in grid.neighbors(pos) {
            if let Some(&n_nutrient) = nutrient_map.get(&npos) {
                let diff = n_nutrient - my_nutrient;
                let dir = (npos - pos).as_vec2();
                gradient += dir * diff;
            }
        }

        tile.nutrient_gradient = if gradient.length_squared() > 0.001 {
            gradient.normalize()
        } else {
            Vec2::ZERO
        };
    }
}

pub fn nutrient_production_system(
    tiles: Query<(&GridPos, &Tile)>,
    mut region_states: ResMut<RegionStates>,
) {
    let mut organic_per_region: HashMap<RegionId, u32> = HashMap::new();
    let mut tiles_per_region: HashMap<RegionId, u32> = HashMap::new();

    for (_, tile) in tiles.iter() {
        if let Occupant::Player(rid) = tile.occupant {
            *tiles_per_region.entry(rid).or_insert(0) += 1;
            if matches!(tile.contents, Some(TileContents::OrganicMatter)) {
                *organic_per_region.entry(rid).or_insert(0) += 1;
            }
        }
    }

    for (rid, state) in &mut region_states.regions {
        let tile_count = tiles_per_region.get(rid).copied().unwrap_or(0);
        let organic = organic_per_region.get(rid).copied().unwrap_or(0);

        let production = match state.specialization {
            Some(SpecializationType::Decomposer) => organic as f32 * 2.0 + tile_count as f32 * 0.1,
            Some(SpecializationType::Symbiont) => tile_count as f32 * 0.5,
            _ => tile_count as f32 * 0.05,
        };

        let consumption = tile_count as f32 * 0.2;

        state.nutrients += production - consumption;
        state.nutrients = state.nutrients.max(0.0);

        state.energy += (production * 0.1).min(5.0);
    }
}

pub fn nutrient_transport_system(
    tiles: Query<(&GridPos, &Tile)>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    let mut transporter_neighbors: HashMap<RegionId, Vec<RegionId>> = HashMap::new();

    for (gpos, tile) in tiles.iter() {
        if let Occupant::Player(rid) = tile.occupant {
            let is_transporter = region_states
                .get(rid)
                .map(|r| r.specialization == Some(SpecializationType::Transporter))
                .unwrap_or(false);
            if !is_transporter {
                continue;
            }

            for (_npos, nentity) in grid.neighbors(gpos.0) {
                if let Ok((_, ntile)) = tiles.get(nentity)
                    && let Occupant::Player(nrid) = ntile.occupant
                    && nrid != rid
                {
                    transporter_neighbors.entry(rid).or_default().push(nrid);
                }
            }
        }
    }

    let nutrient_snapshot: HashMap<RegionId, f32> = region_states
        .regions
        .iter()
        .map(|(&rid, state)| (rid, state.nutrients))
        .collect();

    let mut transfers: HashMap<RegionId, f32> = HashMap::new();

    for neighbors in transporter_neighbors.values() {
        if neighbors.len() < 2 {
            continue;
        }

        let neighbor_nutrients: Vec<(RegionId, f32)> = neighbors
            .iter()
            .filter_map(|&nrid| nutrient_snapshot.get(&nrid).map(|&n| (nrid, n)))
            .collect();

        if neighbor_nutrients.is_empty() {
            continue;
        }

        let avg: f32 = neighbor_nutrients.iter().map(|(_, n)| n).sum::<f32>()
            / neighbor_nutrients.len() as f32;

        for &(nrid, nutrients) in &neighbor_nutrients {
            let diff = avg - nutrients;
            let transfer = diff * 0.1;
            if transfer.abs() > 0.01 {
                *transfers.entry(nrid).or_insert(0.0) += transfer;
            }
        }
    }

    for (rid, delta) in transfers {
        if let Some(state) = region_states.get_mut(rid) {
            state.nutrients = (state.nutrients + delta).max(0.0);
        }
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

    fn spawn_tile_at(app: &mut App, pos: IVec2, tile: Tile) -> Entity {
        let entity = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);
        entity
    }

    #[test]
    fn gradient_points_toward_higher_nutrients() {
        let mut app = test_app();

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                nutrient_level: 0.1,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(6, 5),
            Tile {
                nutrient_level: 0.9,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(4, 5),
            Tile {
                nutrient_level: 0.1,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(5, 6),
            Tile {
                nutrient_level: 0.1,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(5, 4),
            Tile {
                nutrient_level: 0.1,
                ..default()
            },
        );

        app.add_systems(Update, nutrient_gradient_system);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let tile = app
            .world()
            .get::<Tile>(grid.tiles[&IVec2::new(5, 5)])
            .expect("tile should exist");
        assert!(
            tile.nutrient_gradient.x > 0.0,
            "gradient x should be positive, got {}",
            tile.nutrient_gradient.x
        );
    }

    #[test]
    fn decomposer_region_produces_nutrients() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).expect("region exists").specialization =
            Some(SpecializationType::Decomposer);
        rs.get_mut(rid).expect("region exists").nutrients = 5.0;

        spawn_tile_at(
            &mut app,
            IVec2::new(0, 0),
            Tile {
                occupant: Occupant::Player(rid),
                contents: Some(TileContents::OrganicMatter),
                ..default()
            },
        );

        app.add_systems(Update, nutrient_production_system);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert!(
            rs.get(rid).expect("region exists").nutrients > 5.0,
            "decomposer should produce nutrients"
        );
    }

    #[test]
    fn transporter_moves_nutrients_between_regions() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rich_rid = rs.create_region();
        rs.get_mut(rich_rid).expect("region exists").nutrients = 100.0;
        let poor_rid = rs.create_region();
        rs.get_mut(poor_rid).expect("region exists").nutrients = 1.0;
        let transport_rid = rs.create_region();
        rs.get_mut(transport_rid)
            .expect("region exists")
            .specialization = Some(SpecializationType::Transporter);

        spawn_tile_at(
            &mut app,
            IVec2::new(0, 0),
            Tile {
                occupant: Occupant::Player(rich_rid),
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(1, 0),
            Tile {
                occupant: Occupant::Player(transport_rid),
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(2, 0),
            Tile {
                occupant: Occupant::Player(poor_rid),
                ..default()
            },
        );

        app.add_systems(Update, nutrient_transport_system);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert!(
            rs.get(poor_rid).expect("region exists").nutrients > 1.0,
            "poor region should receive nutrients via transport"
        );
    }
}
