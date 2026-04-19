use bevy::prelude::*;
use shroom_core::{MutationSelection, SlotMachineTriggered, UnlockOption};

#[derive(Component)]
pub struct SlotMachinePanel;

#[derive(Component)]
pub struct SlotMachineOption {
    pub index: usize,
}

#[derive(Resource, Default)]
pub struct SlotMachineState {
    pub active: bool,
    pub options: Vec<UnlockOption>,
    pub selected: Option<usize>,
}

pub fn slot_machine_ui_system(
    mut commands: Commands,
    mut slot_events: MessageReader<SlotMachineTriggered>,
    mut state: ResMut<SlotMachineState>,
    existing: Query<Entity, With<SlotMachinePanel>>,
) {
    for event in slot_events.read() {
        if event.options.is_empty() {
            continue;
        }
        state.active = true;
        state.options.clone_from(&event.options);
        state.selected = None;

        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }

        commands
            .spawn((
                SlotMachinePanel,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(25.0),
                    top: Val::Percent(25.0),
                    width: Val::Percent(50.0),
                    height: Val::Percent(50.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Choose an Upgrade"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.9, 0.3)),
                ));

                for (i, option) in event.options.iter().enumerate() {
                    parent
                        .spawn((
                            SlotMachineOption { index: i },
                            Node {
                                width: Val::Percent(80.0),
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                            Button,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(format!("{}: {}", option.name, option.description)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                }
            });
    }
}

pub fn slot_machine_selection_system(
    mut commands: Commands,
    interactions: Query<(&Interaction, &SlotMachineOption), Changed<Interaction>>,
    mut state: ResMut<SlotMachineState>,
    mut mutation_selection: ResMut<MutationSelection>,
    panels: Query<Entity, With<SlotMachinePanel>>,
) {
    if !state.active {
        return;
    }

    for (interaction, option) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            state.selected = Some(option.index);
            mutation_selection.selected_index = Some(option.index);
            state.active = false;

            for entity in panels.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}
