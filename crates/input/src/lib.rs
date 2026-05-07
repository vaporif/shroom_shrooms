use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

mod action;
mod camera;
mod selection;
mod speed;
mod wisp;

pub use action::{Action, default_input_map};
pub use camera::{GameCamera, camera_system, spawn_camera};
pub use kingdom_core::SelectedRegion;
pub use selection::selection_system;
pub use speed::speed_input_system;
pub use wisp::{TileTapped, WispPhase, WispState, wisp_input_system};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<Action>::default())
            .insert_resource(default_input_map())
            .init_resource::<ActionState<Action>>()
            .init_resource::<WispState>()
            .add_message::<TileTapped>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    camera_system,
                    wisp_input_system,
                    selection_system,
                    speed_input_system,
                ),
            );
    }
}
