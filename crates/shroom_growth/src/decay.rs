use bevy::prelude::*;
use shroom_core::*;

pub fn decay_system(
    _commands: Commands,
    mut tiles: Query<(Entity, &GridPos, &mut Tile)>,
    region_states: Res<RegionStates>,
) {
    for (_entity, _gpos, mut tile) in tiles.iter_mut() {
        if let Occupant::Player(rid) = tile.occupant {
            let starved = region_states.get(rid).is_none_or(|r| r.nutrients <= 0.0);

            if starved {
                tile.biomass -= 0.1;
                if tile.biomass <= 0.0 {
                    tile.biomass = 0.0;
                    tile.occupant = Occupant::Empty;
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
    fn starved_tile_loses_biomass() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().nutrients = 0.0;

        let entity = app
            .world_mut()
            .spawn((
                GridPos(IVec2::new(0, 0)),
                Tile {
                    occupant: Occupant::Player(rid),
                    biomass: 1.0,
                    ..default()
                },
            ))
            .id();

        app.add_systems(Update, decay_system);
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(
            tile.biomass < 1.0,
            "biomass should decay when region is starved"
        );
    }

    #[test]
    fn zero_biomass_tile_reverts_to_empty() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().nutrients = 0.0;

        let entity = app
            .world_mut()
            .spawn((
                GridPos(IVec2::new(0, 0)),
                Tile {
                    occupant: Occupant::Player(rid),
                    biomass: 0.05,
                    ..default()
                },
            ))
            .id();

        app.add_systems(Update, decay_system);
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(
            tile.occupant,
            Occupant::Empty,
            "tile should revert to empty at zero biomass"
        );
    }
}
