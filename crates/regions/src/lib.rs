use bevy::prelude::*;

use fungai_core::SimulationSet;

mod discovery;
mod fragment;
mod mutation;
mod slot_machine;
mod specialization;

pub use discovery::{
    DecompProgress, StudyProgress, decomposer_discovery_system, explorer_discovery_system,
    researcher_study_system,
};
pub use fragment::fragment_system;
pub use mutation::{AppliedMutations, mutation_system};
pub use slot_machine::{SlotMachineRng, slot_machine_system};
pub use specialization::specialization_system;

pub struct RegionsPlugin;

impl Plugin for RegionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StudyProgress>()
            .init_resource::<DecompProgress>()
            .init_resource::<SlotMachineRng>()
            .init_resource::<AppliedMutations>()
            .add_systems(
                Update,
                (
                    specialization_system,
                    explorer_discovery_system,
                    researcher_study_system,
                    decomposer_discovery_system,
                    slot_machine_system,
                    mutation_system,
                    fragment_system,
                )
                    .chain()
                    .in_set(SimulationSet),
            );
    }
}
