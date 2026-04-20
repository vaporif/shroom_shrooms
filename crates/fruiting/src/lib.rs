mod effects;
mod fruiting;
mod spores;

use bevy::prelude::*;

use fungai_core::SimulationSet;

pub use effects::mufungai_effect_system;
pub use fruiting::fruiting_system;
pub use spores::{SporeRng, spore_system};

pub struct FruitingPlugin;

impl Plugin for FruitingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SporeRng>().add_systems(
            Update,
            (
                fruiting::fruiting_system,
                effects::mufungai_effect_system,
                spores::spore_system,
            )
                .chain()
                .in_set(SimulationSet),
        );
    }
}
