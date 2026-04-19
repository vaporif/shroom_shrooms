use bevy::prelude::*;
use shroom_core::{RegionStates, SPEC_TIER_1};

const INVESTMENT_RATE: f32 = 2.0;

pub fn specialization_system(mut region_states: ResMut<RegionStates>) {
    for (_rid, state) in &mut region_states.regions {
        let Some(target) = state.target_specialization else {
            continue;
        };

        if let Some(current) = state.specialization {
            if current != target {
                state.specialization = None;
                state.specialization_investment = 0.0;
            }
        }

        let invest_amount = INVESTMENT_RATE.min(state.nutrients);
        if invest_amount <= 0.0 {
            continue;
        }
        state.nutrients -= invest_amount;
        state.specialization_investment += invest_amount;

        if state.specialization_investment >= SPEC_TIER_1 && state.specialization.is_none() {
            state.specialization = Some(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::SpecializationType;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<RegionStates>();
        app
    }

    #[test]
    fn region_invests_nutrients_toward_target() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        let state = rs.get_mut(rid).unwrap();
        state.target_specialization = Some(SpecializationType::Explorer);
        state.nutrients = 50.0;

        app.add_systems(Update, specialization_system);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        let state = rs.get(rid).unwrap();
        assert!(state.specialization_investment > 0.0);
        assert!(state.nutrients < 50.0);
    }

    #[test]
    fn tier_1_unlocks_at_threshold() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        let state = rs.get_mut(rid).unwrap();
        state.target_specialization = Some(SpecializationType::Decomposer);
        state.specialization_investment = 99.0;
        state.nutrients = 50.0;

        app.add_systems(Update, specialization_system);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        let state = rs.get(rid).unwrap();
        assert!(state.specialization_investment >= SPEC_TIER_1);
        assert_eq!(state.specialization, Some(SpecializationType::Decomposer));
    }

    #[test]
    fn changing_target_resets_investment() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        let state = rs.get_mut(rid).unwrap();
        state.target_specialization = Some(SpecializationType::Explorer);
        state.specialization_investment = 50.0;
        state.specialization = Some(SpecializationType::Explorer);
        state.target_specialization = Some(SpecializationType::Parasite);

        app.add_systems(Update, specialization_system);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        let state = rs.get(rid).unwrap();
        assert_eq!(
            state.target_specialization,
            Some(SpecializationType::Parasite)
        );
        assert!(state.specialization_investment < 50.0);
    }
}
