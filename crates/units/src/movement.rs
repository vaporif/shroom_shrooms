use bevy::prelude::*;
use kingdom_core::{GridPos, SimulationSpeed, UNIT_SPEED_HEXES_PER_SEC, UnitMovement};

pub fn unit_movement_system(
    time: Res<Time>,
    speed: Res<SimulationSpeed>,
    mut units: Query<(&mut GridPos, &mut UnitMovement)>,
) {
    let speed_mult = match *speed {
        SimulationSpeed::Paused => return,
        SimulationSpeed::Normal => 1.0,
        SimulationSpeed::Fast => 2.0,
        SimulationSpeed::Fastest => 4.0,
    };
    // Cap catch-up to one hex per frame so a frame hitch (breakpoint, window
    // drag) can't make a unit teleport past intermediate tiles uninterpolated.
    let step = (UNIT_SPEED_HEXES_PER_SEC * speed_mult * time.delta_secs()).min(1.0);

    for (mut gpos, mut movement) in &mut units {
        if movement.path.is_empty() {
            continue;
        }
        movement.edge_progress += step;
        while movement.edge_progress >= 1.0 && !movement.path.is_empty() {
            let next = movement.path.remove(0);
            gpos.0 = next;
            movement.edge_progress -= 1.0;
        }
        if movement.path.is_empty() {
            movement.edge_progress = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use hexx::Hex;
    use kingdom_core::{Unit, UnitKind};

    /// Each `app.update()` advances `Time` by a fixed, deterministic delta — no
    /// wall-clock `sleep`, so the test is fast and immune to a loaded CI box.
    fn test_app(speed: SimulationSpeed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(speed);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.add_systems(Update, unit_movement_system);
        app
    }

    fn spawn_unit(app: &mut App, path: Vec<Hex>) -> Entity {
        app.world_mut()
            .spawn((
                GridPos(Hex::new(0, 0)),
                Unit {
                    kind: UnitKind::Founder,
                    owner: kingdom_core::RegionId(0),
                },
                UnitMovement {
                    path,
                    edge_progress: 0.0,
                },
            ))
            .id()
    }

    #[test]
    fn unit_advances_along_its_path() {
        let mut app = test_app(SimulationSpeed::Normal);
        let unit = spawn_unit(&mut app, vec![Hex::new(1, 0), Hex::new(2, 0)]);
        // UNIT_SPEED_HEXES_PER_SEC = 1.0, 100ms/frame at Normal speed: ~11
        // frames cross one hex. 20 frames is a comfortable margin.
        for _ in 0..20 {
            app.update();
        }
        let gpos = app.world().get::<GridPos>(unit).unwrap();
        assert_ne!(gpos.0, Hex::new(0, 0), "unit moved off its start hex");
    }

    #[test]
    fn unit_does_not_advance_while_paused() {
        let mut app = test_app(SimulationSpeed::Paused);
        let unit = spawn_unit(&mut app, vec![Hex::new(1, 0)]);
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(app.world().get::<GridPos>(unit).unwrap().0, Hex::new(0, 0));
        assert_eq!(
            app.world().get::<UnitMovement>(unit).unwrap().edge_progress,
            0.0
        );
    }
}
