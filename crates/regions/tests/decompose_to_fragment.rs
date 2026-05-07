use bevy::prelude::*;
use kingdom_core::*;
use kingdom_regions::fragment_system;

#[test]
fn fragment_fuses_when_covered() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<GameState>();
    // FragmentFused is also registered by CorePlugin in real builds; this test
    // uses MinimalPlugins, so the registration is needed here. Harmless.
    app.add_message::<FragmentFused>();
    app.add_systems(Update, fragment_system);

    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let pos = Hex::new(0, 0);
    let fragment_id = FragmentId(0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::Fragment(fragment_id)),
                ..default()
            },
            FragmentAgent {
                fragment_id,
                fused: false,
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);
    app.world_mut().resource_mut::<GameState>().fragments_total = 1;

    app.update();

    let agent = app.world().get::<FragmentAgent>(e).unwrap();
    assert!(agent.fused);
    assert_eq!(app.world().resource::<GameState>().fragments_fused, 1);
}
