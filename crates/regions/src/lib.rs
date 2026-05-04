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
pub use mutation::{AppliedMutations, MutationSelection, mutation_system};
pub use slot_machine::{SlotMachineRng, SlotMachineTriggered, slot_machine_system};
pub use specialization::specialization_system;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum RegionsSystems {
    Specialization,
    Discovery,
    Unlock,
    Fragment,
}

pub struct SpecializationPlugin;

impl Plugin for SpecializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            specialization_system.in_set(RegionsSystems::Specialization),
        );
    }
}

pub struct DiscoveryPlugin;

impl Plugin for DiscoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StudyProgress>()
            .init_resource::<DecompProgress>()
            .add_systems(
                Update,
                (
                    explorer_discovery_system,
                    researcher_study_system,
                    decomposer_discovery_system,
                )
                    .chain()
                    .in_set(RegionsSystems::Discovery),
            );
    }
}

pub struct UnlockPlugin;

impl Plugin for UnlockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SlotMachineRng>()
            .init_resource::<AppliedMutations>()
            .init_resource::<MutationSelection>()
            .add_systems(
                Update,
                (slot_machine_system, mutation_system)
                    .chain()
                    .in_set(RegionsSystems::Unlock),
            );
    }
}

pub struct FragmentPlugin;

impl Plugin for FragmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fragment_system.in_set(RegionsSystems::Fragment));
    }
}

pub struct RegionsPlugin;

impl Plugin for RegionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SlotMachineTriggered>()
            .configure_sets(
                Update,
                (RegionsSystems::Discovery, RegionsSystems::Unlock)
                    .chain()
                    .in_set(SimulationSet),
            )
            .configure_sets(
                Update,
                (RegionsSystems::Specialization, RegionsSystems::Fragment).in_set(SimulationSet),
            )
            .add_plugins((
                SpecializationPlugin,
                DiscoveryPlugin,
                UnlockPlugin,
                FragmentPlugin,
            ));
    }
}
