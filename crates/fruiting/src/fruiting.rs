use bevy::prelude::*;
use kingdom_core::{FragmentAgent, FruitingBody, GameState, MushroomEntity, RegionStates};

pub fn fruiting_system(
    mut commands: Commands,
    mut fruiting_bodies: Query<(Entity, &mut FruitingBody)>,
    mut region_states: ResMut<RegionStates>,
    mut game_state: ResMut<GameState>,
    fragments: Query<&FragmentAgent>,
) {
    let progress_rate = 0.1;
    let sugar_cost = 5.0;

    for (entity, mut body) in fruiting_bodies.iter_mut() {
        let fragment_fused = fragments
            .iter()
            .any(|f| f.fragment_id == body.fragment_id && f.fused);
        if !fragment_fused {
            continue;
        }

        let has_resources = region_states
            .get(body.region_id)
            .is_some_and(|r| r.sugars >= sugar_cost);
        if !has_resources {
            continue;
        }

        if let Some(state) = region_states.get_mut(body.region_id) {
            state.sugars -= sugar_cost;
        }
        body.progress += progress_rate;

        if body.progress >= 1.0 {
            commands.spawn(MushroomEntity {
                fragment_id: body.fragment_id,
                pos: body.column_top,
                vision_radius: 10.0,
            });
            game_state.mushrooms_fruited += 1;
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{FragmentId, GridWorld, Hex, RegionStates};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<GameState>();
        app
    }

    #[test]
    fn fruiting_body_gains_progress_with_resources() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        {
            let mut rs = app.world_mut().resource_mut::<RegionStates>();
            let state = rs.get_mut(rid).unwrap();
            state.sugars = 50.0;
        }

        app.world_mut().spawn(FruitingBody {
            region_id: rid,
            fragment_id: FragmentId(0),
            progress: 0.0,
            column_top: Hex::new(5, 10),
        });

        app.world_mut().spawn(FragmentAgent {
            fragment_id: FragmentId(0),
            fused: true,
        });

        app.add_systems(Update, fruiting_system);
        app.update();

        let fb = app
            .world_mut()
            .query::<&FruitingBody>()
            .single(app.world())
            .expect("should have exactly one fruiting body");
        assert!(fb.progress > 0.0, "fruiting body should gain progress");
    }

    #[test]
    fn fruiting_completes_and_spawns_mushroom() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        {
            let mut rs = app.world_mut().resource_mut::<RegionStates>();
            let state = rs.get_mut(rid).unwrap();
            state.sugars = 100.0;
        }

        app.world_mut().spawn(FruitingBody {
            region_id: rid,
            fragment_id: FragmentId(0),
            progress: 0.95,
            column_top: Hex::new(5, 10),
        });
        app.world_mut().spawn(FragmentAgent {
            fragment_id: FragmentId(0),
            fused: true,
        });

        {
            let mut gs = app.world_mut().resource_mut::<GameState>();
            gs.mushrooms_required = 1;
            gs.fragments_total = 1;
            gs.fragments_fused = 1;
        }

        app.add_systems(Update, fruiting_system);
        app.update();

        let mushroom_count = app
            .world_mut()
            .query::<&MushroomEntity>()
            .iter(app.world())
            .count();
        assert_eq!(
            mushroom_count, 1,
            "completed fruiting should spawn mushroom"
        );
    }
}
