use std::collections::HashMap;

use bevy::prelude::*;
use kingdom_core::{
    GridPos, GridWorld, Hex, MIN_TRADE_MOISTURE, MOISTURE_COST_PER_SUGAR, PlantRootAgent,
    RegionStates, SUGAR_FROM_SYMBIOSIS, Tile, TileContents,
};

pub fn symbiosis_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut plants: Query<&mut PlantRootAgent>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    let snapshot: HashMap<Hex, Option<TileContents>> =
        tiles.iter().map(|(gp, t)| (gp.0, t.contents)).collect();

    for (gpos, mut tile) in tiles.iter_mut() {
        if !tile.is_owned() {
            continue;
        }
        if tile.moisture <= MIN_TRADE_MOISTURE {
            continue;
        }
        let Some(rid) = tile.region_id else { continue };

        for (npos, nentity) in grid.neighbors(gpos.0) {
            let Some(&n_contents) = snapshot.get(&npos) else {
                continue;
            };
            if !matches!(n_contents, Some(TileContents::PlantRoot(_))) {
                continue;
            }
            // Skip if the plant agent was despawned but TileContents wasn't cleared yet.
            let Ok(mut plant) = plants.get_mut(nentity) else {
                continue;
            };
            plant.trade_active = true;
            if let Some(state) = region_states.get_mut(rid) {
                state.sugars += SUGAR_FROM_SYMBIOSIS;
            }
            tile.moisture =
                (tile.moisture - SUGAR_FROM_SYMBIOSIS * MOISTURE_COST_PER_SUGAR).max(0.0);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{Hex, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.insert_resource(create_hex_layout());
        app.add_systems(Update, symbiosis_system);
        app
    }

    #[test]
    fn adjacent_plant_root_yields_sugars() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let center = Hex::new(0, 0);
        let neighbor = center.all_neighbors()[0];
        let myc = app
            .world_mut()
            .spawn((
                GridPos(center),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    moisture: 1.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(center, myc);
        let plant = app
            .world_mut()
            .spawn((
                GridPos(neighbor),
                Tile {
                    contents: Some(TileContents::PlantRoot(0)),
                    ..default()
                },
                PlantRootAgent {
                    plant_id: 0,
                    health: 1.0,
                    trade_active: false,
                    nutrient_intake: 0.0,
                    sugar_output: 0.0,
                    neglect_timer: 0,
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(neighbor, plant);
        let before = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        app.update();
        let after = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        assert!(after > before, "{before} -> {after}");
        assert!(
            app.world()
                .get::<PlantRootAgent>(plant)
                .unwrap()
                .trade_active
        );
    }

    #[test]
    fn low_moisture_blocks_trade() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let center = Hex::new(0, 0);
        let neighbor = center.all_neighbors()[0];
        let myc = app
            .world_mut()
            .spawn((
                GridPos(center),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    moisture: 0.1,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(center, myc);
        app.world_mut().spawn((
            GridPos(neighbor),
            Tile {
                contents: Some(TileContents::PlantRoot(0)),
                ..default()
            },
            PlantRootAgent {
                plant_id: 0,
                health: 1.0,
                trade_active: false,
                nutrient_intake: 0.0,
                sugar_output: 0.0,
                neglect_timer: 0,
            },
        ));
        let before = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        app.update();
        let after = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        assert_eq!(before, after);
    }
}
