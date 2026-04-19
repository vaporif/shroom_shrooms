mod effects;
mod fruiting;
mod spores;

use bevy::prelude::*;

pub use effects::mushroom_effect_system;
pub use fruiting::fruiting_system;
pub use spores::{spore_system, SporeRng};

pub struct FruitingPlugin;

impl Plugin for FruitingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SporeRng>().add_systems(
            Update,
            (
                fruiting::fruiting_system,
                effects::mushroom_effect_system,
                spores::spore_system,
            )
                .chain(),
        );
    }
}
