use bevy::prelude::*;
use shroom_core::*;

pub fn neutral_fungi_system(
    mut fungi: Query<(&GridPos, &mut NeutralFungusAgent)>,
    tiles: Query<&Tile>,
    grid: Res<GridWorld>,
) {
    for (gpos, mut fungus) in fungi.iter_mut() {
        if let Some(&entity) = grid.tiles.get(&gpos.0)
            && let Ok(tile) = tiles.get(entity)
            && tile.occupant.is_player()
        {
            fungus.merge_progress += 0.05;
        }
    }
}

pub fn plant_system(
    mut plants: Query<(&GridPos, &mut PlantRootAgent)>,
    tiles: Query<&Tile>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    for (gpos, mut plant) in plants.iter_mut() {
        if plant.health <= 0.0 {
            continue;
        }

        let mut symbiont_rid = None;
        for (_npos, nentity) in grid.neighbors(gpos.0) {
            if let Ok(ntile) = tiles.get(nentity)
                && let Occupant::Player(rid) = ntile.occupant
            {
                let is_symbiont = region_states
                    .get(rid)
                    .is_some_and(|r| r.specialization == Some(SpecializationType::Symbiont));
                if is_symbiont {
                    symbiont_rid = Some(rid);
                    break;
                }
            }
        }

        if let Some(rid) = symbiont_rid {
            if !plant.trade_active {
                plant.trade_active = true;
                plant.neglect_timer = 0;
            }
            if let Some(state) = region_states.get_mut(rid) {
                let nutrient_cost = 0.5;
                if state.nutrients >= nutrient_cost {
                    state.nutrients -= nutrient_cost;
                    plant.nutrient_intake += nutrient_cost;
                    let sugar = nutrient_cost * 1.5;
                    state.energy += sugar;
                    plant.sugar_output = sugar;
                    plant.neglect_timer = 0;
                } else {
                    plant.neglect_timer += 1;
                }
            }
        } else if plant.trade_active {
            plant.neglect_timer += 1;
            if plant.neglect_timer > TRADE_LINK_NEGLECT_LIMIT {
                plant.trade_active = false;
            }
        }
    }
}

pub fn fauna_system(
    mut commands: Commands,
    mut fauna: Query<(Entity, &mut GridPos, &FaunaAgent)>,
    mut tiles: Query<&mut Tile>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
) {
    for (entity, mut gpos, agent) in fauna.iter_mut() {
        let pos = gpos.0;

        if let Some(&tile_entity) = grid.tiles.get(&pos)
            && let Ok(tile) = tiles.get(tile_entity)
            && let Occupant::Player(rid) = tile.occupant
        {
            let is_hunter = region_states
                .get(rid)
                .is_some_and(|r| r.specialization == Some(SpecializationType::Hunter));
            if is_hunter {
                commands.entity(entity).despawn();
                if let Ok(mut tile) = tiles.get_mut(tile_entity) {
                    tile.contents = Some(TileContents::OrganicMatter);
                }
                continue;
            }
        }

        if let Some(&tile_entity) = grid.tiles.get(&pos)
            && let Ok(mut tile) = tiles.get_mut(tile_entity)
            && (tile.occupant.is_player() || tile.occupant.is_rival())
        {
            tile.biomass = (tile.biomass - agent.damage_per_tick).max(0.0);
        }

        let new_pos = pos + IVec2::new(0, -1);
        if grid.tiles.contains_key(&new_pos) {
            gpos.0 = new_pos;
        } else {
            commands.entity(entity).despawn();
        }
    }
}

pub fn bacteria_system(
    mut commands: Commands,
    mut bacteria: Query<(&GridPos, &mut BacteriaColonyAgent)>,
    mut tiles: Query<&mut Tile>,
    grid: Res<GridWorld>,
) {
    for (gpos, mut colony) in bacteria.iter_mut() {
        if let Some(&tile_entity) = grid.tiles.get(&gpos.0)
            && let Ok(mut tile) = tiles.get_mut(tile_entity)
        {
            tile.nutrient_level = (tile.nutrient_level - 0.05).max(0.0);

            if tile.nutrient_level <= 0.01 {
                colony.spread_timer += 1;
                if colony.spread_timer >= colony.spread_interval {
                    colony.spread_timer = 0;
                    for (npos, nentity) in grid.neighbors(gpos.0) {
                        if let Ok(ntile) = tiles.get(nentity)
                            && ntile.nutrient_level > 0.1
                            && ntile.biomass < BACTERIA_BIOMASS_BLOCK_THRESHOLD
                        {
                            commands.spawn((
                                GridPos(npos),
                                BacteriaColonyAgent {
                                    spread_timer: 0,
                                    spread_interval: colony.spread_interval,
                                },
                            ));
                            break;
                        }
                    }
                }
            }
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

    #[test]
    fn plant_trade_produces_energy() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().specialization = Some(SpecializationType::Symbiont);
        rs.get_mut(rid).unwrap().nutrients = 20.0;

        let pos = IVec2::new(5, 5);
        let tile_e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, tile_e);

        let plant_pos = IVec2::new(6, 5);
        let plant_tile = app
            .world_mut()
            .spawn((GridPos(plant_pos), Tile::default()))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(plant_pos, plant_tile);

        app.world_mut().spawn((
            GridPos(plant_pos),
            PlantRootAgent {
                plant_id: 0,
                health: 1.0,
                trade_active: false,
                nutrient_intake: 0.0,
                sugar_output: 0.0,
                neglect_timer: 0,
            },
        ));

        app.add_systems(Update, plant_system);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert!(
            rs.get(rid).unwrap().energy > 0.0,
            "symbiont should gain energy from trade"
        );
    }

    #[test]
    fn hunter_traps_fauna() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().specialization = Some(SpecializationType::Hunter);

        let pos = IVec2::new(3, 3);
        let tile_e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, tile_e);

        let below = app
            .world_mut()
            .spawn((GridPos(IVec2::new(3, 2)), Tile::default()))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(IVec2::new(3, 2), below);

        app.world_mut().spawn((
            GridPos(pos),
            FaunaAgent {
                health: 1.0,
                damage_per_tick: 0.1,
            },
        ));

        app.add_systems(Update, fauna_system);
        app.update();

        let fauna_count = app
            .world_mut()
            .query::<&FaunaAgent>()
            .iter(app.world())
            .count();
        assert_eq!(fauna_count, 0, "fauna should be trapped by hunter");
    }

    #[test]
    fn bacteria_drains_nutrients() {
        let mut app = test_app();
        let pos = IVec2::new(2, 2);
        let tile_e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    nutrient_level: 0.5,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, tile_e);

        app.world_mut().spawn((
            GridPos(pos),
            BacteriaColonyAgent {
                spread_timer: 0,
                spread_interval: 10,
            },
        ));

        app.add_systems(Update, bacteria_system);
        app.update();

        let tile = app.world().get::<Tile>(tile_e).unwrap();
        assert!(tile.nutrient_level < 0.5, "bacteria should drain nutrients");
    }
}
