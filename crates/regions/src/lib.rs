use bevy::prelude::*;

use kingdom_core::SimulationSystems;

mod discovery;
mod fragment;
mod mutation;
mod slot_machine;

pub use discovery::{DecompProgress, decomposition_system};
pub use fragment::fragment_system;
pub use mutation::{AppliedMutations, MutationSelection, mutation_system};
pub use slot_machine::{SlotMachineRng, SlotMachineTriggered, slot_machine_system};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum RegionsSystems {
    Discovery,
    Unlock,
    Fragment,
}

pub struct DiscoveryPlugin;

impl Plugin for DiscoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DecompProgress>().add_systems(
            Update,
            decomposition_system.in_set(RegionsSystems::Discovery),
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
                    .in_set(SimulationSystems),
            )
            .configure_sets(Update, RegionsSystems::Fragment.in_set(SimulationSystems))
            .add_plugins((DiscoveryPlugin, UnlockPlugin, FragmentPlugin));
    }
}
