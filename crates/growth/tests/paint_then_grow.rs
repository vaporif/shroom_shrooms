//! Proves: writing positive priority_bias on a frontier tile causes biomass
//! to spread preferentially toward the bias direction over multiple ticks.

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
    app.insert_resource(DensityFlowRng(StdRng::seed_from_u64(42)));
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

fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
    let e = app.world_mut().spawn((GridPos(pos), tile)).id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);
    e
}

#[test]
fn paint_then_grow_biases_outflow() {
    let mut app = test_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let layout = create_hex_layout();
    let center = Hex::new(10, 10);
    let neighbors = center.all_neighbors();
    let target = neighbors[0];
    let opposite = neighbors[3];
    let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

    spawn(
        &mut app,
        center,
        Tile {
            region_id: Some(rid),
            biomass: 1.5,
            moisture: 1.0,
            priority_bias: dir * 1.0,
            ..default()
        },
    );
    for &n in &neighbors {
        spawn(
            &mut app,
            n,
            Tile {
                moisture: 0.6,
                ..default()
            },
        );
    }

    for _ in 0..15 {
        app.update();
    }

    let grid = app.world().resource::<GridWorld>();
    let target_b = app
        .world()
        .get::<Tile>(grid.tiles[&target])
        .unwrap()
        .biomass;
    let opposite_b = app
        .world()
        .get::<Tile>(grid.tiles[&opposite])
        .unwrap()
        .biomass;

    assert!(
        target_b > opposite_b * 1.5,
        "biased target ({target_b}) should outpace opposite ({opposite_b}) by >1.5x"
    );
}
