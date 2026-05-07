use std::collections::HashMap;

use bevy::prelude::*;
use kingdom_core::{GridPos, GridWorld, Hex, HexLayout, Tile};

pub fn nutrient_gradient_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    layout: Res<HexLayout>,
) {
    let nutrient_map: HashMap<Hex, f32> = tiles
        .iter()
        .map(|(gp, t)| (gp.0, t.soil_richness))
        .collect();

    for (gpos, mut tile) in tiles.iter_mut() {
        let pos = gpos.0;
        let mut gradient = Vec2::ZERO;
        let my_nutrient = tile.soil_richness;
        let from_world = layout.hex_to_world_pos(pos);

        for (npos, _) in grid.neighbors(pos) {
            if let Some(&n_nutrient) = nutrient_map.get(&npos) {
                let diff = n_nutrient - my_nutrient;
                let to_world = layout.hex_to_world_pos(npos);
                let dir = (to_world - from_world).normalize_or_zero();
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

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{RegionStates, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.insert_resource(create_hex_layout());
        app
    }

    fn spawn_tile_at(app: &mut App, pos: Hex, tile: Tile) -> Entity {
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

        let center = Hex::new(5, 5);
        let neighbors = center.all_neighbors();
        let rich_neighbor = neighbors[0];

        spawn_tile_at(
            &mut app,
            center,
            Tile {
                soil_richness: 0.1,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            rich_neighbor,
            Tile {
                soil_richness: 0.9,
                ..default()
            },
        );
        for &n in &neighbors[1..] {
            spawn_tile_at(
                &mut app,
                n,
                Tile {
                    soil_richness: 0.1,
                    ..default()
                },
            );
        }

        app.add_systems(Update, nutrient_gradient_system);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let tile = app
            .world()
            .get::<Tile>(grid.tiles[&center])
            .expect("tile should exist");

        let layout = app.world().resource::<HexLayout>();
        let expected_dir =
            (layout.hex_to_world_pos(rich_neighbor) - layout.hex_to_world_pos(center)).normalize();
        let dot = tile.nutrient_gradient.dot(expected_dir);
        assert!(
            dot > 0.0,
            "gradient should point toward higher nutrients, dot={dot}"
        );
    }
}
