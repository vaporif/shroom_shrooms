use bevy::prelude::*;
use shroom_core::{GameState, RegionStates};

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct HudRegionText;

#[derive(Component)]
pub struct HudTurnText;

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
                Text::new("No region selected"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

pub fn update_hud(
    game_state: Res<GameState>,
    region_states: Res<RegionStates>,
    mut turn_text: Query<&mut Text, With<HudTurnText>>,
    mut region_text: Query<&mut Text, (With<HudRegionText>, Without<HudTurnText>)>,
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
        if let Some((_rid, state)) = region_states.regions.iter().next() {
            let spec_name = state
                .specialization
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "Unspecialized".into());
            **text = format!(
                "{} | N:{:.0} E:{:.0} B:{:.0} | Tiles:{}",
                spec_name, state.nutrients, state.energy, state.biomass, state.tile_count
            );
        }
    }
}
