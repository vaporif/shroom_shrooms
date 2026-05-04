use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use fungai_core::{
    AbilityEffectType, ActiveAbilityEffects, ActiveEffect, MushroomEntity, RegionStates,
    SpecializationType, SporeAction,
};
use fungai_input::SelectedRegion;

#[derive(Component)]
pub struct AbilityBarRoot;

#[derive(Component)]
pub struct AbilityButton {
    pub ability_index: usize,
}

#[derive(Component)]
pub struct SporeButton;

pub fn spawn_ability_bar(mut commands: Commands) {
    commands.spawn((
        AbilityBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Percent(30.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        },
    ));
}

#[derive(SystemParam)]
pub struct AbilityBarEntities<'w, 's> {
    bar: Query<'w, 's, Entity, With<AbilityBarRoot>>,
    existing_buttons: Query<'w, 's, Entity, With<AbilityButton>>,
    existing_spore: Query<'w, 's, Entity, With<SporeButton>>,
}

pub fn update_ability_bar(
    region_states: Res<RegionStates>,
    selected: Res<SelectedRegion>,
    spore_action: Res<SporeAction>,
    mushrooms: Query<&MushroomEntity>,
    entities: AbilityBarEntities,
    mut commands: Commands,
) {
    for entity in entities.existing_buttons.iter() {
        commands.entity(entity).despawn();
    }
    for entity in entities.existing_spore.iter() {
        commands.entity(entity).despawn();
    }

    let Ok(bar_entity) = entities.bar.single() else {
        return;
    };

    // Use the selected region to find the right one
    let region_info = selected.region_id.and_then(|rid| {
        region_states.get(rid).and_then(|state| {
            if state.specialization.is_some() && state.tier() >= 2 {
                Some((rid, state))
            } else {
                None
            }
        })
    });

    if let Some((_rid, state)) = region_info {
        let ability_name = match state.specialization {
            Some(SpecializationType::Decomposer) => "Enzyme Burst",
            Some(SpecializationType::Parasite) => "Toxin Release",
            Some(SpecializationType::Symbiont) => "Root Shield",
            Some(SpecializationType::Hunter) => "Snare Network",
            Some(SpecializationType::Explorer) => "Probe Burst",
            Some(SpecializationType::Transporter) => "Emergency Relay",
            Some(SpecializationType::Infiltrator) => "Surprise Emerge",
            Some(SpecializationType::Researcher) => "Trigger Study",
            None => return,
        };

        commands.entity(bar_entity).with_children(|parent| {
            parent
                .spawn((
                    AbilityButton { ability_index: 0 },
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.4)),
                    Button,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(ability_name),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
    }

    // Spore button when mushrooms exist
    if mushrooms.iter().next().is_some() {
        let label = if spore_action.cooldown_remaining > 0 {
            format!("Spores ({})", spore_action.cooldown_remaining)
        } else {
            "Release Spores".into()
        };

        commands.entity(bar_entity).with_children(|parent| {
            parent
                .spawn((
                    SporeButton,
                    Node {
                        width: Val::Px(110.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.15, 0.4)),
                    Button,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
    }
}

pub fn ability_click_system(
    interactions: Query<(&Interaction, &AbilityButton), Changed<Interaction>>,
    selected: Res<SelectedRegion>,
    mut region_states: ResMut<RegionStates>,
    mut effects: ResMut<ActiveAbilityEffects>,
) {
    for (interaction, _button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(rid) = selected.region_id else {
            continue;
        };

        let (has_ability, energy, spec) = {
            let Some(state) = region_states.get(rid) else {
                continue;
            };
            if state.tier() < 2 || state.energy < 10.0 {
                continue;
            }
            (true, state.energy, state.specialization)
        };

        if !has_ability {
            continue;
        }

        let effect_type = match spec {
            Some(SpecializationType::Decomposer) => AbilityEffectType::DoubleNutrientProduction,
            Some(SpecializationType::Parasite) => AbilityEffectType::StealBiomass,
            Some(SpecializationType::Explorer) => AbilityEffectType::RevealRadius,
            Some(SpecializationType::Symbiont) => AbilityEffectType::DoubleTradeEnergy,
            Some(SpecializationType::Hunter) => AbilityEffectType::KillFauna,
            Some(SpecializationType::Infiltrator) => AbilityEffectType::InfiltrateRival,
            Some(SpecializationType::Transporter) => AbilityEffectType::DoubleTransport,
            Some(SpecializationType::Researcher) => AbilityEffectType::DoubleStudySpeed,
            None => continue,
        };

        if let Some(state) = region_states.get_mut(rid) {
            state.energy = energy - 10.0;
        }

        effects.effects.push(ActiveEffect {
            region_id: rid,
            effect_type,
            ticks_remaining: 5,
        });
    }
}

pub fn spore_button_system(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SporeButton>)>,
    mut spore_action: ResMut<SporeAction>,
) {
    for interaction in interactions.iter() {
        if *interaction == Interaction::Pressed && spore_action.cooldown_remaining == 0 {
            spore_action.triggered = true;
        }
    }
}
