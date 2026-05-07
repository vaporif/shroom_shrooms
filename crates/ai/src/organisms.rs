use bevy::ecs::message::{Message, MessageWriter};
use bevy::prelude::*;
use kingdom_core::*;

#[derive(Message)]
pub struct NeutralFungiMerged {
    pub fungus_id: u32,
    pub region_id: RegionId,
}

/// Pick the neighbor with the lowest world-space y (i.e. "downward").
fn downward_neighbor(pos: Hex, layout: &HexLayout) -> Hex {
    pos.all_neighbors()
        .into_iter()
        .min_by(|a, b| {
            let ay = layout.hex_to_world_pos(*a).y;
            let by = layout.hex_to_world_pos(*b).y;
            ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(pos)
}

pub fn neutral_fungi_system(
    mut commands: Commands,
    mut fungi: Query<(Entity, &GridPos, &mut NeutralFungusAgent)>,
    tiles: Query<&Tile>,
    grid: Res<GridWorld>,
    mut merged_messages: MessageWriter<NeutralFungiMerged>,
) {
    for (entity, gpos, mut fungus) in fungi.iter_mut() {
        if let Some(&tile_entity) = grid.tiles.get(&gpos.0)
            && let Ok(tile) = tiles.get(tile_entity)
            && let Some(rid) = tile.region_id
        {
            fungus.merge_progress += 0.05;

            if fungus.merge_progress >= 1.0 {
                merged_messages.write(NeutralFungiMerged {
                    fungus_id: fungus.fungus_id,
                    region_id: rid,
                });
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn plant_system(mut plants: Query<&mut PlantRootAgent>) {
    for mut plant in plants.iter_mut() {
        if plant.health <= 0.0 {
            continue;
        }

        if plant.trade_active {
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
    layout: Res<HexLayout>,
) {
    for (entity, mut gpos, agent) in fauna.iter_mut() {
        let pos = gpos.0;

        if let Some(&tile_entity) = grid.tiles.get(&pos)
            && let Ok(mut tile) = tiles.get_mut(tile_entity)
            && tile.region_id.is_some()
        {
            tile.biomass = (tile.biomass - agent.damage_per_tick).max(0.0);
        }

        let new_pos = downward_neighbor(pos, &layout);
        if new_pos != pos && grid.tiles.contains_key(&new_pos) {
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
            tile.soil_richness = (tile.soil_richness - 0.05).max(0.0);

            if tile.soil_richness <= 0.01 {
                colony.spread_timer += 1;
                if colony.spread_timer >= colony.spread_interval {
                    colony.spread_timer = 0;
                    for (npos, nentity) in grid.neighbors(gpos.0) {
                        if let Ok(ntile) = tiles.get(nentity)
                            && ntile.soil_richness > 0.1
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
        app.insert_resource(create_hex_layout());
        app
    }

    #[test]
    fn bacteria_drains_nutrients() {
        let mut app = test_app();
        let pos = Hex::new(2, 2);
        let tile_e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    soil_richness: 0.5,
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
        assert!(tile.soil_richness < 0.5, "bacteria should drain nutrients");
    }
}
