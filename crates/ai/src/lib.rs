use bevy::prelude::*;

use kingdom_core::SimulationSystems;

mod environment;
mod organisms;

pub use environment::{EnvironmentRng, environment_threat_system};
pub use organisms::{
    NeutralFungiMerged, bacteria_system, fauna_system, neutral_fungi_system, plant_system,
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AiSystems {
    Organisms,
    Environment,
}

pub struct OrganismsPlugin;

impl Plugin for OrganismsPlugin {
    fn build(&self, app: &mut App) {
        // The binary registers OrganismsPlugin directly rather than going through
        // AiPlugin, so the message has to be added here — otherwise Bevy panics on
        // the first write. SimulationSystems gating must also live here for the
        // same reason: without it `bacteria_system` runs at frame rate and bacteria
        // colonies double every fraction of a second.
        app.add_message::<NeutralFungiMerged>()
            .configure_sets(Update, AiSystems::Organisms.in_set(SimulationSystems))
            .add_systems(
                Update,
                (
                    neutral_fungi_system,
                    plant_system,
                    fauna_system,
                    bacteria_system,
                )
                    .chain()
                    .in_set(AiSystems::Organisms),
            );
    }
}

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentRng>()
            .configure_sets(Update, AiSystems::Environment.in_set(SimulationSystems))
            .add_systems(
                Update,
                environment_threat_system.in_set(AiSystems::Environment),
            );
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (AiSystems::Organisms, AiSystems::Environment)
                .chain()
                .in_set(SimulationSystems),
        )
        .add_plugins((OrganismsPlugin, EnvironmentPlugin));
    }
}
