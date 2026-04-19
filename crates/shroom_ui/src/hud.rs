use bevy::prelude::*;
use shroom_core::{GameState, HintsVisible, RegionStates, SimulationSpeed};
use shroom_input::SelectedRegion;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct HudRegionText;

#[derive(Component)]
pub struct HudTurnText;

#[derive(Component)]
pub struct SpeedDisplayText;

#[derive(Component)]
pub struct HintsPanel;

pub fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                HudTurnText,
                Text::new("Turn: 0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                HudRegionText,
                Text::new("Click a tile to inspect."),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });

    // Speed display (bottom-right)
    commands.spawn((
        SpeedDisplayText,
        Text::new(SimulationSpeed::default().label()),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
    ));

    // Hints panel (top-right)
    commands
        .spawn((
            HintsPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            let hints = [
                "WASD \u{2014} Pan camera",
                "Scroll \u{2014} Zoom",
                "Left click \u{2014} Select tile",
                "Right drag \u{2014} Set growth priority",
                "Space \u{2014} Pause  |  +/- Speed",
                "1-8 \u{2014} Set specialization",
                "H \u{2014} Hide hints",
            ];
            for hint in hints {
                parent.spawn((
                    Text::new(hint),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
            }
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update_hud(
    game_state: Res<GameState>,
    region_states: Res<RegionStates>,
    selected: Res<SelectedRegion>,
    speed: Res<SimulationSpeed>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut hints_visible: ResMut<HintsVisible>,
    mut turn_text: Query<&mut Text, With<HudTurnText>>,
    mut region_text: Query<
        &mut Text,
        (
            With<HudRegionText>,
            Without<HudTurnText>,
            Without<SpeedDisplayText>,
        ),
    >,
    mut speed_text: Query<
        &mut Text,
        (
            With<SpeedDisplayText>,
            Without<HudTurnText>,
            Without<HudRegionText>,
        ),
    >,
    mut hints_panels: Query<&mut Visibility, With<HintsPanel>>,
) {
    if let Ok(mut text) = turn_text.single_mut() {
        **text = format!(
            "Turn: {} | Fragments: {}/{} | Mushrooms: {}/{}",
            game_state.turn,
            game_state.fragments_fused,
            game_state.fragments_total,
            game_state.mushrooms_fruited,
            game_state.mushrooms_required,
        );
    }

    if let Ok(mut text) = region_text.single_mut() {
        let state = selected.region_id.and_then(|rid| region_states.get(rid));

        match state {
            Some(state) => {
                let spec_name = state
                    .specialization
                    .map(|s| format!("{s:?}"))
                    .unwrap_or_else(|| {
                        state
                            .target_specialization
                            .map(|t| format!("-> {t:?}"))
                            .unwrap_or_else(|| "Unspecialized".into())
                    });
                **text = format!(
                    "{} | N:{:.0} E:{:.0} B:{:.0} | Tiles:{} | Inv:{:.0}",
                    spec_name,
                    state.nutrients,
                    state.energy,
                    state.biomass,
                    state.tile_count,
                    state.specialization_investment
                );
            }
            None => {
                **text = "Click a tile to inspect.".into();
            }
        }
    }

    // Update speed display
    if let Ok(mut text) = speed_text.single_mut() {
        **text = speed.label().into();
    }

    // Toggle hints on H key
    if keyboard.just_pressed(KeyCode::KeyH) {
        hints_visible.0 = !hints_visible.0;
    }

    if let Ok(mut vis) = hints_panels.single_mut() {
        *vis = if hints_visible.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
