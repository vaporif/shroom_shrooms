use bevy::prelude::*;

mod camera;
mod priority;
mod selection;
mod specialization_input;
mod speed;

pub use camera::{camera_system, spawn_camera, GameCamera};
pub use priority::priority_system;
pub use selection::{selection_system, SelectedRegion};
pub use specialization_input::specialization_input_system;
pub use speed::speed_input_system;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedRegion>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    camera_system,
                    selection_system,
                    priority_system,
                    speed_input_system,
                    specialization_input_system,
                ),
            );
    }
}
