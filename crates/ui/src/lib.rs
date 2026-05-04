use bevy::prelude::*;

mod ability_bar;
pub mod game_screens;
mod hud;
mod slot_machine_ui;
mod spec_picker;

pub use ability_bar::{
    ability_click_system, spawn_ability_bar, spore_button_system, update_ability_bar,
    AbilityBarRoot, AbilityButton, ActiveAbilityEffects, SporeButton,
};
pub use hud::{spawn_hud, update_hud, HintsVisible};
pub use slot_machine_ui::{
    slot_machine_selection_system, slot_machine_ui_system, SlotMachineState,
};
pub use spec_picker::{spec_picker_click_system, spec_picker_highlight_system, spec_picker_system};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HintsVisible>()
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, update_hud);
    }
}

pub struct AbilityBarPlugin;

impl Plugin for AbilityBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveAbilityEffects>()
            .add_systems(Startup, spawn_ability_bar)
            .add_systems(
                Update,
                (
                    update_ability_bar,
                    ability_click_system,
                    spore_button_system,
                ),
            );
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

pub struct SpecPickerPlugin;

impl Plugin for SpecPickerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spec_picker_system,
                spec_picker_click_system,
                spec_picker_highlight_system,
            ),
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
        app.add_plugins((
            HudPlugin,
            AbilityBarPlugin,
            SlotMachineUiPlugin,
            SpecPickerPlugin,
            GameScreensPlugin,
        ));
    }
}
