use bevy::prelude::*;
use kingdom_core::{HiveCaptured, SimulationSystems};
use kingdom_world::region_tracking_system;

mod hive;

pub use hive::hive_capture_system;

pub struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HiveCaptured>().add_systems(
            Update,
            hive_capture_system
                .in_set(SimulationSystems)
                .after(region_tracking_system),
        );
    }
}
