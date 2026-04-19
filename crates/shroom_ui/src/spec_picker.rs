use bevy::prelude::*;
use shroom_core::{RegionStates, SpecializationType};
use shroom_input::SelectedRegion;

#[derive(Component)]
pub struct SpecPickerPanel;

#[derive(Component)]
pub struct SpecPickerButton {
    pub spec: SpecializationType,
}

const SPECS: [(SpecializationType, &str, Color); 8] = [
    (
        SpecializationType::Decomposer,
        "Decomposer",
        Color::srgb(0.2, 0.7, 0.3),
    ),
    (
        SpecializationType::Parasite,
        "Parasite",
        Color::srgb(0.8, 0.2, 0.2),
    ),
    (
        SpecializationType::Symbiont,
        "Symbiont",
        Color::srgb(0.3, 0.8, 0.8),
    ),
    (
        SpecializationType::Explorer,
        "Explorer",
        Color::srgb(1.0, 0.9, 0.3),
    ),
    (
        SpecializationType::Hunter,
        "Hunter",
        Color::srgb(0.6, 0.4, 0.1),
    ),
    (
        SpecializationType::Transporter,
        "Transporter",
        Color::srgb(0.9, 0.6, 0.2),
    ),
    (
        SpecializationType::Infiltrator,
        "Infiltrator",
        Color::srgb(0.6, 0.3, 0.8),
    ),
    (
        SpecializationType::Researcher,
        "Researcher",
        Color::srgb(0.3, 0.5, 0.9),
    ),
];

/// Show the picker when a player region is selected, hide when nothing is selected.
pub fn spec_picker_system(
    mut commands: Commands,
    selected: Res<SelectedRegion>,
    region_states: Res<RegionStates>,
    existing: Query<Entity, With<SpecPickerPanel>>,
) {
    let should_show = selected
        .region_id
        .and_then(|rid| region_states.get(rid).map(|state| (rid, state)));

    // Despawn if nothing selected or region doesn't exist
    let Some((_rid, _state)) = should_show else {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    // Already showing
    if !existing.is_empty() {
        return;
    }

    // Spawn picker panel on the right side
    commands
        .spawn((
            SpecPickerPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Percent(30.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Specialize:"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.6)),
            ));

            for (spec, name, color) in &SPECS {
                parent
                    .spawn((
                        SpecPickerButton { spec: *spec },
                        Node {
                            width: Val::Px(130.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(color.with_alpha(0.6)),
                        Button,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(*name),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
        });
}

/// Handle clicks on specialization buttons.
pub fn spec_picker_click_system(
    mut commands: Commands,
    interactions: Query<(&Interaction, &SpecPickerButton), Changed<Interaction>>,
    selected: Res<SelectedRegion>,
    mut region_states: ResMut<RegionStates>,
    panels: Query<Entity, With<SpecPickerPanel>>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(rid) = selected.region_id else {
            continue;
        };
        let Some(state) = region_states.get_mut(rid) else {
            continue;
        };

        state.target_specialization = Some(button.spec);

        // Close the panel after selection
        for entity in panels.iter() {
            commands.entity(entity).despawn();
        }
    }
}
