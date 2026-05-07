use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use kingdom_core::*;
use kingdom_regions::{DecompProgress, decomposition_system, slot_machine_system};

#[test]
fn unique_decomp_to_slot_machine_pipeline() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<DecompProgress>();
    app.init_resource::<kingdom_regions::SlotMachineRng>();
    app.add_message::<DecompositionComplete>();
    app.add_message::<kingdom_regions::SlotMachineTriggered>();

    let captured = std::sync::Arc::new(std::sync::Mutex::new(0));
    let captured_c = captured.clone();
    app.add_systems(
        Update,
        (
            decomposition_system,
            slot_machine_system,
            (move |mut r: MessageReader<kingdom_regions::SlotMachineTriggered>| {
                for ev in r.read() {
                    if ev.options.len() == 3 {
                        *captured_c.lock().unwrap() += 1;
                    }
                }
            }),
        )
            .chain(),
    );

    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let pos = Hex::new(0, 0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::UniqueDecomposable(0)),
                ..default()
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);

    app.world_mut()
        .resource_mut::<DecompProgress>()
        .entries
        .insert(pos, 0.99);

    app.update();
    app.update(); // event delivery may take a frame.

    assert_eq!(*captured.lock().unwrap(), 1);
}
