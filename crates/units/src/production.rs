use bevy::prelude::*;
use kingdom_core::{
    GridPos, HIVE_PRODUCTION_RATE, HIVE_PRODUCTION_SUGAR_COST, Hive, RegionStates, UNIT_CAP_BASE,
    UNIT_CAP_PER_HIVE, UNIT_UPKEEP_SUGAR, Unit, UnitKind, UnitMovement,
};

pub fn hive_production_system(
    mut commands: Commands,
    mut hives: Query<(&GridPos, &mut Hive)>,
    units: Query<&Unit>,
    mut region_states: ResMut<RegionStates>,
) {
    let captured_hives = hives
        .iter()
        .filter(|(_, h)| h.captured_by.is_some())
        .count() as u32;
    // The cap is global across all the player's networks.
    let cap = UNIT_CAP_BASE + captured_hives * UNIT_CAP_PER_HIVE;
    let mut living = units.iter().count() as u32;

    for (gpos, mut hive) in &mut hives {
        let Some(owner) = hive.captured_by else {
            continue;
        };
        if living >= cap {
            continue; // capped: production stalls, no sugars drained
        }
        let Some(state) = region_states.get_mut(owner) else {
            continue;
        };
        if state.sugars <= 0.0 {
            continue; // no sugars: production stalls
        }
        state.sugars = (state.sugars - HIVE_PRODUCTION_SUGAR_COST).max(0.0);
        hive.production += HIVE_PRODUCTION_RATE;
        if hive.production >= 1.0 {
            hive.production = 0.0;
            commands.spawn((
                GridPos(gpos.0),
                Unit {
                    kind: UnitKind::Founder,
                    owner,
                },
                UnitMovement::default(),
            ));
            living += 1; // re-check the cap across hives finishing the same tick
        }
    }
}

pub fn unit_upkeep_system(units: Query<&Unit>, mut region_states: ResMut<RegionStates>) {
    for unit in &units {
        if let Some(state) = region_states.get_mut(unit.owner) {
            state.sugars = (state.sugars - UNIT_UPKEEP_SUGAR).max(0.0);
        }
        // A unit whose owner region no longer exists pays no upkeep.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GridPos, Hive, RegionStates, Unit, UnitKind};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<RegionStates>();
        app.add_systems(Update, (hive_production_system, unit_upkeep_system).chain());
        app
    }

    #[test]
    fn captured_hive_with_sugars_spawns_a_founder() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(rid)
            .unwrap()
            .sugars = 100.0;
        app.world_mut().spawn((
            GridPos(hexx::Hex::new(0, 0)),
            Hive {
                captured_by: Some(rid),
                production: 0.95,
            },
        ));
        app.update();
        let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
        assert_eq!(founders, 1);
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
    fn production_stalls_at_the_unit_cap() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(rid)
            .unwrap()
            .sugars = 100.0;
        // One captured hive → cap is UNIT_CAP_BASE + 1 * UNIT_CAP_PER_HIVE = 4.
        // Pre-spawn 4 units so the hive starts already at the cap.
        for _ in 0..4 {
            app.world_mut().spawn((
                GridPos(hexx::Hex::new(9, 9)),
                Unit {
                    kind: UnitKind::Founder,
                    owner: rid,
                },
            ));
        }
        app.world_mut().spawn((
            GridPos(hexx::Hex::new(0, 0)),
            Hive {
                captured_by: Some(rid),
                production: 0.99,
            },
        ));
        let sugars_before = app
            .world()
            .resource::<RegionStates>()
            .get(rid)
            .unwrap()
            .sugars;
        app.update();
        let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
        assert_eq!(founders, 4, "no founder spawned beyond the cap");
        // Production drained no sugars while capped; only upkeep on the 4 units does.
        let drained = sugars_before
            - app
                .world()
                .resource::<RegionStates>()
                .get(rid)
                .unwrap()
                .sugars;
        assert!(
            (drained - 0.4).abs() < 1e-4,
            "only upkeep drained, not production"
        );
    }

    #[test]
    fn production_stalls_when_region_has_no_sugars() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        // Owner region is broke: the `sugars <= 0.0` early-continue must fire.
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(rid)
            .unwrap()
            .sugars = 0.0;
        let hive = app
            .world_mut()
            .spawn((
                GridPos(hexx::Hex::new(0, 0)),
                Hive {
                    captured_by: Some(rid),
                    production: 0.95,
                },
            ))
            .id();
        app.update();
        let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
        assert_eq!(founders, 0, "no founder spawned without sugars");
        assert_eq!(
            app.world().get::<Hive>(hive).unwrap().production,
            0.95,
            "production does not advance while the region is broke",
        );
    }

    #[test]
    fn cap_blocks_second_hive_finishing_same_tick() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(rid)
            .unwrap()
            .sugars = 100.0;
        // Two captured hives → cap is UNIT_CAP_BASE + 2 * UNIT_CAP_PER_HIVE = 6.
        // Pre-spawn 5 units so exactly one slot remains free this tick.
        for _ in 0..5 {
            app.world_mut().spawn((
                GridPos(hexx::Hex::new(9, 9)),
                Unit {
                    kind: UnitKind::Founder,
                    owner: rid,
                },
            ));
        }
        for _ in 0..2 {
            app.world_mut().spawn((
                GridPos(hexx::Hex::new(0, 0)),
                Hive {
                    captured_by: Some(rid),
                    production: 0.99,
                },
            ));
        }
        app.update();
        let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
        assert_eq!(
            founders, 6,
            "the `living += 1` re-check stops the second hive at the cap",
        );
    }

    #[test]
    fn upkeep_drains_and_clamps_at_zero() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        app.world_mut()
            .resource_mut::<RegionStates>()
            .get_mut(rid)
            .unwrap()
            .sugars = 0.05;
        app.world_mut().spawn((
            GridPos(hexx::Hex::new(9, 9)),
            Unit {
                kind: UnitKind::Founder,
                owner: rid,
            },
        ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<RegionStates>()
                .get(rid)
                .unwrap()
                .sugars,
            0.0
        );
    }
}
