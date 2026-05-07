use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use kingdom_core::{GameState, LaunchConfig, RegionStates, SimulationSpeed};
use kingdom_input::SelectedRegion;

#[derive(Resource, Debug, Reflect)]
pub struct HintsVisible(pub bool);

impl Default for HintsVisible {
    fn default() -> Self {
        Self(true)
    }
}

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
                "Click \u{2014} Inspect tile",
                "Click+drag \u{2014} Paint growth direction",
                "Space \u{2014} Pause  |  +/- Speed",
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

type RegionTextFilter = (
    With<HudRegionText>,
    Without<HudTurnText>,
    Without<SpeedDisplayText>,
);

type SpeedTextFilter = (
    With<SpeedDisplayText>,
    Without<HudTurnText>,
    Without<HudRegionText>,
);

#[derive(SystemParam)]
pub struct HudTexts<'w, 's> {
    turn: Query<'w, 's, &'static mut Text, With<HudTurnText>>,
    region: Query<'w, 's, &'static mut Text, RegionTextFilter>,
    speed: Query<'w, 's, &'static mut Text, SpeedTextFilter>,
    hints: Query<'w, 's, &'static mut Visibility, With<HintsPanel>>,
}

#[derive(SystemParam)]
pub struct HudInputs<'w> {
    game_state: Res<'w, GameState>,
    region_states: Res<'w, RegionStates>,
    selected: Res<'w, SelectedRegion>,
    speed: Res<'w, SimulationSpeed>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    config: Res<'w, LaunchConfig>,
    hints_visible: ResMut<'w, HintsVisible>,
}

pub fn update_hud(inputs: HudInputs, mut texts: HudTexts) {
    let HudInputs {
        game_state,
        region_states,
        selected,
        speed,
        keyboard,
        config,
        mut hints_visible,
    } = inputs;

    if let Ok(mut text) = texts.turn.single_mut() {
        **text = format!(
            "Turn: {} | Speed: {} | Fragments: {}/{} | Mushrooms: {}/{} | Seed: {}",
            game_state.turn,
            speed.label(),
            game_state.fragments_fused,
            game_state.fragments_total,
            game_state.mushrooms_fruited,
            game_state.mushrooms_required,
            config.seed,
        );
    }

    if let Ok(mut text) = texts.region.single_mut() {
        let state = selected.region_id.and_then(|rid| region_states.get(rid));

        match state {
            Some(state) => {
                **text = format!(
                    "Region {}\nSugars: {:.0}\nMelanin: {:.0}\nBiomass: {:.0}\nTiles: {}",
                    state.region_id.0,
                    state.sugars,
                    state.melanin,
                    state.total_biomass,
                    state.tile_count,
                );
            }
            None => {
                **text = "Click a tile to inspect.".into();
            }
        }
    }

    // Update speed display
    if let Ok(mut text) = texts.speed.single_mut() {
        **text = speed.label().into();
    }

    // Toggle hints on H key
    if keyboard.just_pressed(KeyCode::KeyH) {
        hints_visible.0 = !hints_visible.0;
    }

    if let Ok(mut vis) = texts.hints.single_mut() {
        *vis = if hints_visible.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
