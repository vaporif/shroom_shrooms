use bevy::prelude::*;
use shroom_core::{RegionStates, SpecializationType};

use crate::SelectedRegion;

/// Keys 1-8 assign a target specialization to the selected region.
pub fn specialization_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedRegion>,
    mut region_states: ResMut<RegionStates>,
) {
    let spec = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(SpecializationType::Decomposer)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(SpecializationType::Parasite)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(SpecializationType::Symbiont)
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        Some(SpecializationType::Explorer)
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        Some(SpecializationType::Hunter)
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        Some(SpecializationType::Transporter)
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        Some(SpecializationType::Infiltrator)
    } else if keyboard.just_pressed(KeyCode::Digit8) {
        Some(SpecializationType::Researcher)
    } else {
        None
    };

    let Some(target) = spec else { return };
    let Some(rid) = selected.region_id else {
        return;
    };
    let Some(state) = region_states.get_mut(rid) else {
        return;
    };

    state.target_specialization = Some(target);
}
