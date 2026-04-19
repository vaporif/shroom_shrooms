use std::collections::HashMap;

use bevy::prelude::*;
use rand::prelude::*;
use rand::rngs::StdRng;
use shroom_core::*;

#[derive(Resource)]
pub struct RivalRng(pub StdRng);

impl Default for RivalRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(99))
    }
}

#[derive(Resource)]
pub struct RivalState {
    pub rival_id: RivalId,
    pub total_biomass: f32,
}

impl Default for RivalState {
    fn default() -> Self {
        Self {
            rival_id: RivalId(0),
            total_biomass: 0.0,
        }
    }
}

pub fn rival_ai_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    rival_state: ResMut<RivalState>,
    mut rng: ResMut<RivalRng>,
) {
    let rid = rival_state.rival_id;

    let tile_data: HashMap<IVec2, (Occupant, f32, TerrainType)> = tiles
        .iter()
        .map(|(gp, t)| (gp.0, (t.occupant, t.nutrient_level, t.terrain)))
        .collect();

    let mut frontier: Vec<(IVec2, IVec2)> = Vec::new();
    for (&pos, &(occupant, _nutrient, _terrain)) in &tile_data {
        if occupant != Occupant::Rival(rid) {
            continue;
        }
        for (npos, _) in grid.neighbors(pos) {
            if let Some(&(nocc, _nnut, nterrain)) = tile_data.get(&npos)
                && nocc == Occupant::Empty
                && nterrain.is_passable()
            {
                frontier.push((pos, npos));
            }
        }
    }

    frontier.shuffle(&mut rng.0);
    for &(_from, target) in frontier.iter().take(3) {
        if let Some(&entity) = grid.tiles.get(&target)
            && let Ok((_, mut tile)) = tiles.get_mut(entity)
            && tile.occupant == Occupant::Empty
        {
            tile.occupant = Occupant::Rival(rid);
            tile.biomass = 1.0;
        }
    }

    for (&pos, &(occupant, _, _)) in &tile_data {
        if occupant != Occupant::Rival(rid) {
            continue;
        }
        let near_player = grid
            .neighbors(pos)
            .any(|(np, _)| tile_data.get(&np).is_some_and(|(o, _, _)| o.is_player()));
        if near_player
            && let Some(&entity) = grid.tiles.get(&pos)
            && let Ok((_, mut tile)) = tiles.get_mut(entity)
        {
            tile.biomass += 0.5;
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
        app.init_resource::<RivalRng>();
        app.insert_resource(RivalState {
            rival_id: RivalId(0),
            total_biomass: 10.0,
        });
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
    fn rival_expands_into_empty_neighbors() {
        let mut app = test_app();
        let rid = RivalId(0);

        spawn_tile_at(
            &mut app,
            IVec2::new(5, 5),
            Tile {
                occupant: Occupant::Rival(rid),
                biomass: 3.0,
                nutrient_level: 0.8,
                ..default()
            },
        );
        spawn_tile_at(&mut app, IVec2::new(6, 5), Tile::default());
        spawn_tile_at(&mut app, IVec2::new(4, 5), Tile::default());
        spawn_tile_at(&mut app, IVec2::new(5, 6), Tile::default());
        spawn_tile_at(&mut app, IVec2::new(5, 4), Tile::default());

        app.add_systems(Update, rival_ai_system);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let neighbors_occupied = [(6, 5), (4, 5), (5, 6), (5, 4)]
            .iter()
            .filter(|&&(x, y)| {
                let e = grid.tiles[&IVec2::new(x, y)];
                app.world().get::<Tile>(e).unwrap().occupant.is_rival()
            })
            .count();
        assert!(
            neighbors_occupied > 0,
            "rival should expand into at least one neighbor"
        );
    }
}
