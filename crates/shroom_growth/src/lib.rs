use bevy::prelude::*;

mod decay;
mod nutrient;
mod tip;

pub use decay::decay_system;
pub use nutrient::{nutrient_gradient_system, nutrient_production_system, nutrient_transport_system};
pub use tip::{hyphal_tip_system, GrowthRng};

pub struct GrowthPlugin;

impl Plugin for GrowthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GrowthRng>().add_systems(
            Update,
            (
                nutrient_gradient_system,
                nutrient_production_system,
                nutrient_transport_system,
                hyphal_tip_system,
                decay_system,
            )
                .chain(),
        );
    }
}
