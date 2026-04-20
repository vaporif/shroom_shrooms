use bevy::prelude::*;
use fungai_core::{
    GridPos, MUSHROOM_MOISTURE_BONUS, MUSHROOM_MOISTURE_RADIUS, MushroomEntity, Occupant,
    RegionStates, Tile,
};

pub fn mufungai_effect_system(
    mushrooms: Query<&MushroomEntity>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut region_states: ResMut<RegionStates>,
) {
    for mushroom in mushrooms.iter() {
        let mut bonus_region = None;

        for (gpos, mut tile) in tiles.iter_mut() {
            let dist = gpos.0.unsigned_distance_to(mushroom.pos);
            if dist <= MUSHROOM_MOISTURE_RADIUS as u32 {
                tile.moisture = (tile.moisture + MUSHROOM_MOISTURE_BONUS * 0.1).min(1.0);
            }

            if !(bonus_region.is_none() && dist <= 3) {
                continue;
            }

            if let Occupant::Player(rid) = tile.occupant {
                bonus_region = Some(rid);
            }
        }

        let Some(rid) = bonus_region else { continue };
        if let Some(state) = region_states.get_mut(rid) {
            state.nutrients += 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fungai_core::{FragmentId, GridWorld, Hex, RegionStates};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app
    }

    fn spawn_tile_at(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        app.world_mut().spawn((GridPos(pos), tile)).id()
    }

    #[test]
    fn mufungai_boosts_moisture_within_radius() {
        let mut app = test_app();

        let mufungai_pos = Hex::new(5, 5);
        // Hex distance 2 -- well within MUSHROOM_MOISTURE_RADIUS (5)
        let near_pos = Hex::new(5, 3);
        // Hex distance 30 -- far outside
        let far_pos = Hex::new(20, 20);

        let near_entity = spawn_tile_at(
            &mut app,
            near_pos,
            Tile {
                moisture: 0.5,
                ..default()
            },
        );
        let far_entity = spawn_tile_at(
            &mut app,
            far_pos,
            Tile {
                moisture: 0.5,
                ..default()
            },
        );

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: mufungai_pos,
            vision_radius: 10.0,
        });

        app.add_systems(Update, mufungai_effect_system);
        app.update();

        let near_tile = app.world().get::<Tile>(near_entity).expect("near tile");
        let far_tile = app.world().get::<Tile>(far_entity).expect("far tile");

        assert!(
            near_tile.moisture > 0.5,
            "nearby tile moisture should increase, got {}",
            near_tile.moisture
        );
        assert!(
            (far_tile.moisture - 0.5).abs() < f32::EPSILON,
            "far tile moisture should stay unchanged, got {}",
            far_tile.moisture
        );
    }

    #[test]
    fn mufungai_grants_nutrient_bonus_to_nearby_player_region() {
        let mut app = test_app();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let initial_nutrients = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .expect("region")
            .nutrients;

        let mufungai_pos = Hex::new(5, 5);
        // Place a player tile at a hex neighbor (distance 1, within bonus radius of 3)
        let neighbor = mufungai_pos.all_neighbors()[0];
        spawn_tile_at(
            &mut app,
            neighbor,
            Tile {
                occupant: Occupant::Player(rid),
                ..default()
            },
        );

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: mufungai_pos,
            vision_radius: 10.0,
        });

        app.add_systems(Update, mufungai_effect_system);
        app.update();

        let nutrients = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .expect("region")
            .nutrients;
        assert!(
            nutrients > initial_nutrients,
            "region nutrients should increase from mushroom bonus, got {nutrients}"
        );
    }

    #[test]
    fn mufungai_moisture_caps_at_one() {
        let mut app = test_app();

        let pos = Hex::new(5, 5);
        let entity = spawn_tile_at(
            &mut app,
            pos,
            Tile {
                moisture: 0.99,
                ..default()
            },
        );

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos,
            vision_radius: 10.0,
        });

        app.add_systems(Update, mufungai_effect_system);
        app.update();

        let tile = app.world().get::<Tile>(entity).expect("tile");
        assert!(
            tile.moisture <= 1.0,
            "moisture should not exceed 1.0, got {}",
            tile.moisture
        );
    }

    #[test]
    fn no_crash_with_no_mushrooms() {
        let mut app = test_app();
        spawn_tile_at(&mut app, Hex::ZERO, Tile::default());
        app.add_systems(Update, mufungai_effect_system);
        app.update();
    }
}
