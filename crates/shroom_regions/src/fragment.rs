use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use shroom_core::{FragmentAgent, FragmentFused, GameState, GridPos, GridWorld, Tile};

pub fn fragment_system(
    mut fragments: Query<(&GridPos, &mut FragmentAgent)>,
    tiles: Query<&Tile>,
    grid: Res<GridWorld>,
    mut game_state: ResMut<GameState>,
    mut fused_messages: MessageWriter<FragmentFused>,
) {
    for (gpos, mut fragment) in fragments.iter_mut() {
        if fragment.fused {
            continue;
        }

        if let Some(&entity) = grid.tiles.get(&gpos.0) {
            if let Ok(tile) = tiles.get(entity) {
                if tile.occupant.is_player() {
                    fragment.fused = true;
                    game_state.fragments_fused += 1;
                    fused_messages.write(FragmentFused {
                        fragment_id: fragment.fragment_id,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::{FragmentId, Occupant, RegionId};

    #[test]
    fn fragment_fuses_when_player_occupies_tile() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<GameState>();
        app.add_message::<FragmentFused>();
        app.add_systems(Update, fragment_system);

        let rid = RegionId(0);
        let pos = IVec2::new(3, 3);
        let tile_entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, tile_entity);

        app.world_mut().spawn((
            GridPos(pos),
            FragmentAgent {
                fragment_id: FragmentId(0),
                fused: false,
            },
        ));

        app.update();

        let gs = app.world().resource::<GameState>();
        assert_eq!(gs.fragments_fused, 1);
    }
}
