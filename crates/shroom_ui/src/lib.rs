use bevy::prelude::*;

mod ability_bar;
pub mod game_screens;
mod hud;
mod slot_machine_ui;
mod spec_picker;

pub use ability_bar::*;
pub use hud::*;
pub use slot_machine_ui::*;
pub use spec_picker::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SlotMachineState>()
            .add_systems(
                Startup,
                (spawn_hud, spawn_ability_bar, game_screens::spawn_title_card),
            )
            .add_systems(
                Update,
                (
                    update_hud,
                    update_ability_bar,
                    slot_machine_ui_system,
                    slot_machine_selection_system,
                    ability_click_system,
                    spore_button_system,
                    game_screens::title_dismiss_system,
                    game_screens::spawn_victory_panel,
                    game_screens::spawn_defeat_panel,
                    game_screens::restart_button_system,
                    game_screens::game_outcome_system,
                    spec_picker_system,
                    spec_picker_click_system,
                    spec_picker_highlight_system,
                ),
            );
    }
}
