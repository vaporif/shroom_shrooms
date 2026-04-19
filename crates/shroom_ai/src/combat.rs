use bevy::prelude::*;
use shroom_core::*;

use std::collections::HashMap;

pub fn combat_resolution_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
) {
    let tile_data: HashMap<IVec2, (Occupant, f32)> = tiles
        .iter()
        .map(|(gp, t)| (gp.0, (t.occupant, t.biomass)))
        .collect();

    let mut flips: Vec<(IVec2, Occupant)> = Vec::new();

    for (&pos, &(occupant, biomass)) in &tile_data {
        if occupant == Occupant::Empty {
            continue;
        }

        for (npos, _) in grid.neighbors(pos) {
            let Some(&(n_occupant, n_biomass)) = tile_data.get(&npos) else {
                continue;
            };

            let is_border = (occupant.is_player() && n_occupant.is_rival())
                || (occupant.is_rival() && n_occupant.is_player());
            if !is_border {
                continue;
            }

            let my_regional_biomass = match occupant {
                Occupant::Player(rid) => region_states.get(rid).map_or(biomass, |r| r.biomass),
                _ => biomass,
            };

            let their_biomass = match n_occupant {
                Occupant::Player(rid) => region_states.get(rid).map_or(n_biomass, |r| r.biomass),
                _ => n_biomass,
            };

            if their_biomass > 0.0 && my_regional_biomass / their_biomass >= BIOMASS_FLIP_RATIO {
                flips.push((npos, occupant));
            }
        }
    }

    for (pos, new_occupant) in flips {
        if let Some(&entity) = grid.tiles.get(&pos)
            && let Ok((_, mut tile)) = tiles.get_mut(entity)
        {
            tile.occupant = new_occupant;
            tile.biomass *= 0.5;
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

    fn spawn_tile_at(app: &mut App, pos: IVec2, tile: Tile) -> Entity {
        let entity = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);
        entity
    }

    #[test]
    fn strong_side_flips_weak_border_tile() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let player_rid = rs.create_region();
        rs.get_mut(player_rid).unwrap().biomass = 20.0;
        let rival_rid = RivalId(0);

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                occupant: Occupant::Player(player_rid),
                biomass: 20.0,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(6, 5),
            Tile {
                occupant: Occupant::Rival(rival_rid),
                biomass: 5.0,
                ..default()
            },
        );

        app.add_systems(Update, combat_resolution_system);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let tile = app
            .world()
            .get::<Tile>(grid.tiles[&IVec2::new(6, 5)])
            .unwrap();
        assert!(
            tile.occupant.is_player(),
            "weak rival tile should flip to player when biomass ratio exceeds {}",
            BIOMASS_FLIP_RATIO
        );
    }

    #[test]
    fn balanced_border_stays_contested() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let player_rid = rs.create_region();

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                occupant: Occupant::Player(player_rid),
                biomass: 10.0,
                ..default()
            },
        );
        spawn_tile_at(
            &mut app,
            IVec2::new(6, 5),
            Tile {
                occupant: Occupant::Rival(RivalId(0)),
                biomass: 8.0,
                ..default()
            },
        );

        app.add_systems(Update, combat_resolution_system);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let tile = app
            .world()
            .get::<Tile>(grid.tiles[&IVec2::new(6, 5)])
            .unwrap();
        assert!(
            tile.occupant.is_rival(),
            "balanced border should remain contested"
        );
    }
}
