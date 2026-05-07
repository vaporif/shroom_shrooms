use bevy::prelude::*;

pub mod game_screens;
mod hud;
mod slot_machine_ui;
mod tile_popover;

pub use hud::{HintsVisible, spawn_hud, update_hud};
pub use slot_machine_ui::{
    SlotMachineState, slot_machine_selection_system, slot_machine_ui_system,
};
pub use tile_popover::update_tile_popover;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HintsVisible>()
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, (update_hud, update_tile_popover));
    }
}

pub struct SlotMachineUiPlugin;

impl Plugin for SlotMachineUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SlotMachineState>().add_systems(
            Update,
            (slot_machine_ui_system, slot_machine_selection_system),
        );
    }
}

pub struct GameScreensPlugin;

impl Plugin for GameScreensPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, game_screens::spawn_title_card)
            .add_systems(
                Update,
                (
                    game_screens::title_dismiss_system,
                    game_screens::spawn_victory_panel,
                    game_screens::spawn_defeat_panel,
                    game_screens::restart_button_system,
                    game_screens::game_outcome_system,
                ),
            );
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((HudPlugin, SlotMachineUiPlugin, GameScreensPlugin));
    }
}
