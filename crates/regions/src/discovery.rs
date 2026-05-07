use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use kingdom_core::{
    DECOMP_RATE, DecompositionComplete, GridPos, Hex, RegionStates, SUGAR_FROM_DECOMP, Tile,
    TileContents,
};

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct DecompProgress {
    pub entries: HashMap<Hex, f32>,
}

pub fn decomposition_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut region_states: ResMut<RegionStates>,
    mut progress: ResMut<DecompProgress>,
    mut decomp_messages: MessageWriter<DecompositionComplete>,
) {
    for (gpos, mut tile) in tiles.iter_mut() {
        if !tile.is_owned() {
            continue;
        }
        let Some(rid) = tile.region_id else { continue };
        let was_unique = match tile.contents {
            Some(TileContents::OrganicMatter) => false,
            Some(TileContents::UniqueDecomposable(_)) => true,
            _ => continue,
        };

        if let Some(state) = region_states.get_mut(rid) {
            state.sugars += SUGAR_FROM_DECOMP * DECOMP_RATE;
        }

        let prog = progress.entries.entry(gpos.0).or_insert(0.0);
        *prog += DECOMP_RATE;
        if *prog >= 1.0 {
            tile.contents = None;
            tile.soil_richness = (tile.soil_richness + 0.2).min(1.0);
            progress.entries.remove(&gpos.0);
            decomp_messages.write(DecompositionComplete {
                pos: gpos.0,
                was_unique,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::GridWorld;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<DecompProgress>();
        app.add_message::<DecompositionComplete>();
        app.add_systems(Update, decomposition_system);
        app
    }

    fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let e = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);
        e
    }

    #[test]
    fn owned_organic_tile_adds_sugars() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::OrganicMatter),
                ..default()
            },
        );
        let before = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        app.update();
        let after = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        assert!(
            after > before,
            "decomposition should yield sugars: {before} -> {after}"
        );
    }

    #[test]
    fn unique_decomposable_completion_fires_was_unique_event() {
        use bevy::ecs::message::MessageReader;
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = Hex::new(2, 2);
        spawn(
            &mut app,
            pos,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::UniqueDecomposable(0)),
                ..default()
            },
        );
        app.world_mut()
            .resource_mut::<DecompProgress>()
            .entries
            .insert(pos, 0.99);
        let captured = std::sync::Arc::new(std::sync::Mutex::new(false));
        let captured_c = captured.clone();
        app.add_systems(
            Update,
            (move |mut r: MessageReader<DecompositionComplete>| {
                for ev in r.read() {
                    if ev.was_unique {
                        *captured_c.lock().unwrap() = true;
                    }
                }
            })
            .after(decomposition_system),
        );
        app.update();
        assert!(*captured.lock().unwrap());
    }

    #[test]
    fn non_owned_tile_no_progress() {
        let mut app = test_app();
        spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                region_id: None,
                biomass: 0.0,
                contents: Some(TileContents::OrganicMatter),
                ..default()
            },
        );
        app.update();
        assert!(app.world().resource::<DecompProgress>().entries.is_empty());
    }

    #[test]
    fn decomposition_progresses_across_ticks() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = Hex::new(3, 3);
        let entity = spawn(
            &mut app,
            pos,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::OrganicMatter),
                ..default()
            },
        );
        // 1/DECOMP_RATE = 50 ticks in real arithmetic; +1 absorbs f32 drift on
        // the running progress sum.
        for _ in 0..51 {
            app.update();
        }
        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(
            tile.contents.is_none(),
            "expected contents cleared after ~50 ticks, was {:?}",
            tile.contents
        );
    }
}
