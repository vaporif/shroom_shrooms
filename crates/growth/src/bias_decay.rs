use bevy::prelude::*;
use kingdom_core::{BIAS_DECAY, BIOMASS_SNAP_EPSILON, Tile};

pub fn bias_decay_system(mut tiles: Query<&mut Tile>) {
    for mut tile in tiles.iter_mut() {
        tile.priority_bias *= BIAS_DECAY;
        if tile.priority_bias.length_squared() < BIOMASS_SNAP_EPSILON * BIOMASS_SNAP_EPSILON {
            tile.priority_bias = Vec2::ZERO;
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
        app.add_systems(Update, bias_decay_system);
        app
    }

    #[test]
    fn nonzero_bias_shrinks_each_tick() {
        let mut app = test_app();
        let entity = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    priority_bias: Vec2::new(1.0, 0.0),
                    ..default()
                },
            ))
            .id();

        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(tile.priority_bias.x < 1.0);
        assert!(tile.priority_bias.x > 0.0);
        assert!((tile.priority_bias.x - BIAS_DECAY).abs() < 1e-6);
    }

    #[test]
    fn tiny_bias_snaps_to_zero() {
        let mut app = test_app();
        let entity = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    priority_bias: Vec2::new(0.0001, 0.0),
                    ..default()
                },
            ))
            .id();

        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::ZERO);
    }
}
