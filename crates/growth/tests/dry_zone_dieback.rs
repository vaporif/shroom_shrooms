use bevy::prelude::*;
use kingdom_core::*;
use kingdom_growth::{
    DensityFlowRng, bias_decay_system, density_flow_system, dieback_system,
    moisture_diffusion_system, nutrient_gradient_system,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.insert_resource(create_hex_layout());
    app.insert_resource(DensityFlowRng(StdRng::seed_from_u64(7)));
    app.add_message::<TileDiscovered>();
    app.add_systems(
        Update,
        (
            bias_decay_system,
            moisture_diffusion_system,
            nutrient_gradient_system,
            density_flow_system,
            dieback_system,
        )
            .chain(),
    );
    app
}

#[test]
fn dry_zone_loses_claim_over_time() {
    let mut app = test_app();
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
                moisture: 0.0,
                ..default()
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);

    for _ in 0..40 {
        app.update();
    }

    let tile = app.world().get::<Tile>(e).unwrap();
    assert_eq!(
        tile.region_id, None,
        "starved tile should de-claim, biomass={}",
        tile.biomass
    );
}
