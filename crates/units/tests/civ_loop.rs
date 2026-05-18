use bevy::prelude::*;
use hexx::Hex;
use kingdom_core::*;
use kingdom_units::{hive_capture_system, hive_production_system, unit_upkeep_system};
use kingdom_world::region_tracking_system;

fn spawn_tile(app: &mut App, pos: Hex, region: Option<RegionId>, biomass: f32) -> Entity {
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: region,
                biomass,
                ..default()
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);
    e
}

fn sim_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<GameState>();
    app.add_message::<HiveCaptured>();
    app.add_systems(
        Update,
        (
            region_tracking_system,
            hive_capture_system,
            hive_production_system,
            unit_upkeep_system,
        )
            .chain(),
    );
    app
}

#[test]
fn grow_to_capture_hive() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let hive_pos = Hex::new(1, 0);
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    // Hive tile starts unowned; growing biomass onto it captures the hive.
    spawn_tile(&mut app, hive_pos, Some(rid), 1.0);
    let hive = app
        .world_mut()
        .spawn((
            GridPos(hive_pos),
            Hive {
                captured_by: None,
                production: 0.0,
            },
        ))
        .id();

    app.update();
    assert_eq!(
        app.world().get::<Hive>(hive).unwrap().captured_by,
        Some(rid)
    );
}

#[test]
fn captured_hive_produces_founder() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut()
        .resource_mut::<RegionStates>()
        .get_mut(rid)
        .unwrap()
        .sugars = 100.0;
    let hive_pos = Hex::new(1, 0);
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    spawn_tile(&mut app, hive_pos, Some(rid), 1.0);
    app.world_mut().spawn((
        GridPos(hive_pos),
        Hive {
            captured_by: None,
            production: 0.0,
        },
    ));

    // HIVE_PRODUCTION_RATE = 0.05 → ~20 ticks per founder. Run 30.
    for _ in 0..30 {
        app.update();
    }
    let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
    assert!(founders >= 1, "captured hive produced a founder");
    assert!(
        app.world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars
            < 100.0
    );
}

#[test]
fn unit_cap_blocks_overproduction() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut()
        .resource_mut::<RegionStates>()
        .get_mut(rid)
        .unwrap()
        .sugars = 1000.0;
    let hive_pos = Hex::new(1, 0);
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    spawn_tile(&mut app, hive_pos, Some(rid), 1.0);
    app.world_mut().spawn((
        GridPos(hive_pos),
        Hive {
            captured_by: None,
            production: 0.0,
        },
    ));

    for _ in 0..400 {
        app.update();
    }
    // One captured hive → cap = UNIT_CAP_BASE + 1 * UNIT_CAP_PER_HIVE = 4.
    let founders = app.world_mut().query::<&Unit>().iter(app.world()).count() as u32;
    assert_eq!(
        founders,
        UNIT_CAP_BASE + UNIT_CAP_PER_HIVE,
        "production stops at the cap"
    );
}

#[test]
fn upkeep_drains_idle_units() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut()
        .resource_mut::<RegionStates>()
        .get_mut(rid)
        .unwrap()
        .sugars = 50.0;
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    for _ in 0..3 {
        app.world_mut().spawn((
            GridPos(Hex::new(9, 9)),
            Unit {
                kind: UnitKind::Founder,
                owner: rid,
            },
            UnitMovement::default(),
        ));
    }
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
    // 3 units * UNIT_UPKEEP_SUGAR (0.1) = 0.3 per tick.
    assert!((before - after - 0.3).abs() < 1e-4);
}

#[test]
fn two_networks_merge_pools_resources() {
    let mut app = sim_app();
    let old = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let young = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut()
        .resource_mut::<RegionStates>()
        .get_mut(old)
        .unwrap()
        .sugars = 20.0;
    app.world_mut()
        .resource_mut::<RegionStates>()
        .get_mut(young)
        .unwrap()
        .sugars = 15.0;
    spawn_tile(&mut app, Hex::new(0, 0), Some(old), 1.0);
    spawn_tile(&mut app, Hex::new(1, 0), Some(young), 1.0);
    spawn_tile(&mut app, Hex::new(2, 0), Some(young), 1.0);

    app.update();
    let rs = app.world().resource::<RegionStates>();
    assert!(rs.get(young).is_none(), "younger network absorbed");
    assert_eq!(rs.get(old).unwrap().sugars, 35.0, "resources pooled");
    assert!(old.0 < young.0, "the lower id is the survivor");
}

#[test]
fn founder_walks_and_founds_network() {
    use kingdom_units::{founding_system, unit_movement_system};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<GameState>();
    app.init_resource::<SelectedUnit>();
    app.init_resource::<FoundNetworkRequest>();
    app.insert_resource(SimulationSpeed::Normal);
    app.add_message::<NetworkFounded>();
    app.add_systems(Update, (unit_movement_system, founding_system));

    // An existing region; it owns no tiles here, so it imposes no
    // MIN_FOUNDING_DISTANCE constraint on the founding site.
    let existing = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    // A passable, unclaimed target hex.
    let site = Hex::new(20, 0);
    spawn_tile(&mut app, site, None, 0.0);

    let founder = app
        .world_mut()
        .spawn((
            GridPos(site),
            Unit {
                kind: UnitKind::Founder,
                owner: existing,
            },
            UnitMovement::default(),
        ))
        .id();
    app.world_mut().resource_mut::<SelectedUnit>().0 = Some(founder);
    app.world_mut().resource_mut::<FoundNetworkRequest>().0 = true;
    app.update();

    assert!(
        app.world().get::<Unit>(founder).is_none(),
        "founder consumed"
    );
    assert!(
        app.world().resource::<SelectedUnit>().0.is_none(),
        "SelectedUnit cleared with the despawned founder",
    );
    // The founded tile is owned by a fresh region, seeded above CLAIM_THRESHOLD.
    let tile_e = app.world().resource::<GridWorld>().tiles[&site];
    let tile = app.world().get::<Tile>(tile_e).unwrap();
    assert!(tile.biomass >= FOUNDER_SEED_BIOMASS);
    let new_rid = tile.region_id.expect("founded tile is owned");
    assert_ne!(new_rid, existing, "a fresh region was created");
    let rs = app.world().resource::<RegionStates>();
    assert_eq!(rs.regions.len(), 2, "a new network exists");
    assert_eq!(rs.get(new_rid).unwrap().sugars, FOUNDER_SEED_SUGARS);
}
