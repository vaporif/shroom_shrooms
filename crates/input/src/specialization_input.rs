use bevy::prelude::*;
use fungai_core::{RegionStates, SpecializationType};
use leafwing_input_manager::prelude::*;

use crate::SelectedRegion;
use crate::action::Action;

/// Keys 1-8 assign a target specialization to the selected region.
pub fn specialization_input_system(
    actions: Res<ActionState<Action>>,
    selected: Res<SelectedRegion>,
    mut region_states: ResMut<RegionStates>,
) {
    const ACTION_SPECS: &[(Action, SpecializationType)] = &[
        (Action::Spec1, SpecializationType::Decomposer),
        (Action::Spec2, SpecializationType::Parasite),
        (Action::Spec3, SpecializationType::Symbiont),
        (Action::Spec4, SpecializationType::Explorer),
        (Action::Spec5, SpecializationType::Hunter),
        (Action::Spec6, SpecializationType::Transporter),
        (Action::Spec7, SpecializationType::Infiltrator),
        (Action::Spec8, SpecializationType::Researcher),
    ];

    let Some(target) = ACTION_SPECS
        .iter()
        .copied()
        .find_map(|(action, spec)| actions.just_pressed(&action).then_some(spec))
    else {
        return;
    };

    let Some(rid) = selected.region_id else {
        return;
    };
    let Some(state) = region_states.get_mut(rid) else {
        return;
    };

    state.target_specialization = Some(target);
}
