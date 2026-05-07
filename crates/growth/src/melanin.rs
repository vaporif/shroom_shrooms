use bevy::prelude::*;
use kingdom_core::{
    BIOMASS_SNAP_EPSILON, MELANIN_FROM_RADIATION, RADIATION_DEPLETION_RATE, RegionStates, Tile,
};

pub fn melanin_system(mut tiles: Query<&mut Tile>, mut region_states: ResMut<RegionStates>) {
    for mut tile in tiles.iter_mut() {
        if !tile.is_owned() {
            continue;
        }
        if tile.radiation <= 0.0 {
            continue;
        }
        let Some(rid) = tile.region_id else { continue };
        let yield_amt = MELANIN_FROM_RADIATION * tile.radiation;
        if let Some(state) = region_states.get_mut(rid) {
            state.melanin += yield_amt;
        }
        tile.radiation = (tile.radiation - yield_amt * RADIATION_DEPLETION_RATE).max(0.0);
        if tile.radiation < BIOMASS_SNAP_EPSILON {
            tile.radiation = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GridPos, GridWorld, Hex};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.add_systems(Update, melanin_system);
        app
    }

    #[test]
    fn radiated_owned_tile_adds_melanin() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut().spawn((
            GridPos(Hex::ZERO),
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                radiation: 0.5,
                ..default()
            },
        ));
        let before = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .melanin;
        app.update();
        let after = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .melanin;
        assert!(after > before);
    }

    #[test]
    fn radiation_depletes_over_time() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    radiation: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        let r = app.world().get::<Tile>(e).unwrap().radiation;
        assert!(r < 0.5);
    }

    #[test]
    fn non_radiated_tile_no_melanin() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut().spawn((
            GridPos(Hex::ZERO),
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                radiation: 0.0,
                ..default()
            },
        ));
        let before = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .melanin;
        app.update();
        let after = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .melanin;
        assert_eq!(before, after);
    }
}
