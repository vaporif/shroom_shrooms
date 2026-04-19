use bevy::prelude::*;

mod combat;
mod environment;
mod organisms;
mod rival;

pub use combat::*;
pub use environment::*;
pub use organisms::*;
pub use rival::*;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RivalRng>()
            .init_resource::<RivalState>()
            .init_resource::<EnvironmentRng>()
            .add_systems(
                Update,
                (
                    rival_ai_system,
                    neutral_fungi_system,
                    plant_system,
                    fauna_system,
                    bacteria_system,
                    environment_threat_system,
                    combat_resolution_system,
                )
                    .chain(),
            );
    }
}
