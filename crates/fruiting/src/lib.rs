mod effects;
mod fruiting;
mod spores;

use bevy::prelude::*;

use kingdom_core::SimulationSystems;

pub use effects::mushroom_effect_system;
pub use fruiting::fruiting_system;
pub use spores::{SporeAction, SporeRng, spore_system};

pub struct FruitingPlugin;

impl Plugin for FruitingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SporeRng>()
            .init_resource::<SporeAction>()
            .add_systems(
                Update,
                (
                    fruiting::fruiting_system,
                    effects::mushroom_effect_system,
                    spores::spore_system,
                )
                    .chain()
                    .in_set(SimulationSystems),
            );
    }
}
