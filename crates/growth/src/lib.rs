use bevy::prelude::*;

use kingdom_core::SimulationSystems;

mod bias_decay;
mod density_flow;
mod dieback;
mod melanin;
mod moisture;
mod nutrient;
mod symbiosis;

pub use bias_decay::bias_decay_system;
pub use density_flow::{DensityFlowRng, density_flow_system};
pub use dieback::dieback_system;
pub use melanin::melanin_system;
pub use moisture::moisture_diffusion_system;
pub use nutrient::nutrient_gradient_system;
pub use symbiosis::symbiosis_system;

pub struct GrowthPlugin;

impl Plugin for GrowthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DensityFlowRng>().add_systems(
            Update,
            (
                bias_decay_system,
                moisture_diffusion_system,
                nutrient_gradient_system,
                density_flow_system,
                dieback_system,
                symbiosis_system,
                melanin_system,
            )
                .chain()
                .in_set(SimulationSystems),
        );
    }
}
