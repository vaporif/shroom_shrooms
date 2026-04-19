use bevy::prelude::*;

mod discovery;
mod fragment;
mod mutation;
mod slot_machine;
mod specialization;

pub use discovery::{
    decomposer_discovery_system, explorer_discovery_system, researcher_study_system,
    DecompProgress, StudyProgress,
};
pub use fragment::fragment_system;
pub use mutation::{mutation_system, AppliedMutations};
pub use slot_machine::{slot_machine_system, SlotMachineRng};
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
                    .chain(),
            );
    }
}
