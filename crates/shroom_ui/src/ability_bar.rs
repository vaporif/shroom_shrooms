use bevy::prelude::*;
use shroom_core::{RegionStates, SpecializationType};

#[derive(Component)]
pub struct AbilityBarRoot;

#[derive(Component)]
pub struct AbilityButton {
    pub ability_index: usize,
}

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

pub fn update_ability_bar(
    region_states: Res<RegionStates>,
    bar: Query<Entity, With<AbilityBarRoot>>,
    mut commands: Commands,
    existing_buttons: Query<Entity, With<AbilityButton>>,
) {
    for entity in existing_buttons.iter() {
        commands.entity(entity).despawn();
    }

    let Some((_rid, state)) = region_states
        .regions
        .iter()
        .find(|(_, s)| s.specialization.is_some() && s.tier() >= 2)
    else {
        return;
    };

    let Ok(bar_entity) = bar.single() else {
        return;
    };

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
