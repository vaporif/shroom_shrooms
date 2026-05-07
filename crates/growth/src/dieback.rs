use bevy::prelude::*;
use kingdom_core::{BIOMASS_SNAP_EPSILON, CLAIM_THRESHOLD, DIEBACK_RATE, DIEBACK_THRESHOLD, Tile};

pub fn dieback_system(mut tiles: Query<&mut Tile>) {
    for mut tile in tiles.iter_mut() {
        if tile.biomass <= 0.0 {
            continue;
        }
        if tile.moisture < DIEBACK_THRESHOLD {
            tile.biomass *= DIEBACK_RATE;
        }
        if tile.biomass < CLAIM_THRESHOLD {
            tile.region_id = None;
        }
        if tile.biomass < BIOMASS_SNAP_EPSILON {
            tile.biomass = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GridPos, GridWorld, Hex, RegionId};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.add_systems(Update, dieback_system);
        app
    }

    #[test]
    fn low_moisture_shrinks_biomass() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 1.0,
                    moisture: 0.0,
                    ..default()
                },
            ))
            .id();
        app.update();
        let tile = app.world().get::<Tile>(e).unwrap();
        assert!(tile.biomass < 1.0);
    }

    #[test]
    fn biomass_below_claim_threshold_clears_region() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 0.1,
                    moisture: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        let tile = app.world().get::<Tile>(e).unwrap();
        assert_eq!(tile.region_id, None);
    }

    #[test]
    fn tiny_biomass_snaps_to_zero() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 0.0001,
                    moisture: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Tile>(e).unwrap().biomass, 0.0);
    }

    #[test]
    fn dry_tile_decays_then_loses_claim() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 0.5,
                    moisture: 0.0,
                    ..default()
                },
            ))
            .id();
        app.update();
        let tile = app.world().get::<Tile>(e).unwrap();
        assert!(tile.biomass < 0.5);
        assert!(tile.biomass >= CLAIM_THRESHOLD);
        assert_eq!(tile.region_id, Some(RegionId(0)));
        for _ in 0..40 {
            app.update();
        }
        let tile = app.world().get::<Tile>(e).unwrap();
        assert!(tile.biomass < CLAIM_THRESHOLD);
        assert_eq!(tile.region_id, None);
    }
}
