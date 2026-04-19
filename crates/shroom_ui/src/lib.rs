use bevy::prelude::*;

mod ability_bar;
mod hud;
mod slot_machine_ui;

pub use ability_bar::*;
pub use hud::*;
pub use slot_machine_ui::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SlotMachineState>()
            .add_systems(Startup, (spawn_hud, spawn_ability_bar))
            .add_systems(
                Update,
                (
                    update_hud,
                    update_ability_bar,
                    slot_machine_ui_system,
                    slot_machine_selection_system,
                ),
            );
    }
}
