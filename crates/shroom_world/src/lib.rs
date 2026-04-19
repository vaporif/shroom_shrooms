use bevy::prelude::*;

mod region_tracking;
mod terrain_gen;

pub use region_tracking::region_tracking_system;
pub use terrain_gen::{terrain_generation, TerrainSeed};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainSeed>()
            .add_systems(Startup, terrain_generation)
            .add_systems(Update, region_tracking_system);
    }
}
