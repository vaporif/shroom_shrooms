use bevy::prelude::*;
use kingdom_core::{GridPos, GridWorld, Hive, HiveCaptured, Tile};

pub fn hive_capture_system(
    mut hives: Query<(&GridPos, &mut Hive)>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut captured: MessageWriter<HiveCaptured>,
) {
    for (gpos, mut hive) in &mut hives {
        let new_owner = grid
            .tiles
            .get(&gpos.0)
            .and_then(|&e| tiles.get(e).ok())
            .filter(|t| t.is_owned())
            .and_then(|t| t.region_id);

        if new_owner != hive.captured_by {
            if let Some(region_id) = new_owner {
                captured.write(HiveCaptured {
                    hive_pos: gpos.0,
                    region_id,
                });
            }
            hive.captured_by = new_owner;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GameState, RegionId, RegionStates};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<GameState>();
        app.add_message::<kingdom_core::HiveCaptured>();
        app.add_systems(Update, hive_capture_system);
        app
    }

    fn spawn_tile(app: &mut App, pos: hexx::Hex, region: Option<RegionId>, biomass: f32) {
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: region,
                    biomass,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);
    }

    #[test]
    fn hive_on_owned_tile_is_captured() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = hexx::Hex::new(3, 3);
        spawn_tile(&mut app, pos, Some(rid), 1.0);
        let hive = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Hive {
                    captured_by: None,
                    production: 0.0,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Hive>(hive).unwrap().captured_by,
            Some(rid)
        );
    }

    #[test]
    fn hive_on_unowned_tile_is_neutral() {
        let mut app = test_app();
        let pos = hexx::Hex::new(4, 4);
        spawn_tile(&mut app, pos, None, 0.0);
        let hive = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Hive {
                    captured_by: Some(RegionId(7)),
                    production: 0.0,
                },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Hive>(hive).unwrap().captured_by, None);
    }
}
