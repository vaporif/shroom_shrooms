use bevy::prelude::*;
use kingdom_core::{GamePhase, GameState};

#[derive(Component)]
pub struct TitleCard;

#[derive(Component)]
pub struct EndGamePanel;

#[derive(Component)]
pub struct RestartButton;

pub fn game_outcome_system(mut phase: ResMut<GamePhase>, game_state: Res<GameState>) {
    if *phase != GamePhase::Playing {
        return;
    }

    if game_state.victory() {
        *phase = GamePhase::Victory;
    }
}

pub fn spawn_title_card(mut commands: Commands) {
    commands
        .spawn((
            TitleCard,
            GlobalZIndex(100),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("The Fifth Kingdom"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new("Click anywhere to begin"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

pub fn title_dismiss_system(
    mut commands: Commands,
    mut phase: ResMut<GamePhase>,
    mouse: Res<ButtonInput<MouseButton>>,
    title_cards: Query<Entity, With<TitleCard>>,
) {
    if *phase != GamePhase::Title {
        return;
    }

    if mouse.just_pressed(MouseButton::Left) {
        *phase = GamePhase::Playing;
        for entity in &title_cards {
            commands.entity(entity).despawn();
        }
    }
}

pub fn spawn_victory_panel(
    mut commands: Commands,
    phase: Res<GamePhase>,
    existing: Query<Entity, With<EndGamePanel>>,
) {
    if *phase != GamePhase::Victory || !existing.is_empty() {
        return;
    }

    spawn_end_panel(
        &mut commands,
        "Victory!",
        "Your mycelium network is complete.\nThe ancient fragments are reunited.",
        "Play Again",
    );
}

pub fn spawn_defeat_panel(
    mut commands: Commands,
    phase: Res<GamePhase>,
    existing: Query<Entity, With<EndGamePanel>>,
) {
    if *phase != GamePhase::Defeat || !existing.is_empty() {
        return;
    }

    spawn_end_panel(
        &mut commands,
        "Defeat",
        "Rival fungi have overwhelmed the cavern.\nYour network could not survive.",
        "Try Again",
    );
}

fn spawn_end_panel(commands: &mut Commands, heading: &str, message: &str, button_label: &str) {
    commands
        .spawn((
            EndGamePanel,
            GlobalZIndex(100),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(heading),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new(message),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
            parent
                .spawn((
                    RestartButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(button_label),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn restart_button_system(
    mut phase: ResMut<GamePhase>,
    query: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            *phase = GamePhase::Restarting;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn setup_world() -> World {
        let mut world = World::new();
        world.init_resource::<GamePhase>();
        world.init_resource::<GameState>();
        world
    }

    #[test]
    fn victory_triggers_when_conditions_met() {
        let mut world = setup_world();

        *world.resource_mut::<GamePhase>() = GamePhase::Playing;
        let mut gs = world.resource_mut::<GameState>();
        gs.fragments_total = 3;
        gs.fragments_fused = 3;
        gs.mushrooms_required = 2;
        gs.mushrooms_fruited = 2;

        let _ = world.run_system_once(game_outcome_system);

        assert_eq!(*world.resource::<GamePhase>(), GamePhase::Victory);
    }

    #[test]
    fn game_stays_playing_when_no_condition_met() {
        let mut world = setup_world();

        *world.resource_mut::<GamePhase>() = GamePhase::Playing;
        let mut gs = world.resource_mut::<GameState>();
        gs.fragments_total = 3;
        gs.fragments_fused = 1;
        gs.mushrooms_required = 2;
        gs.mushrooms_fruited = 0;
        gs.turn = 5;

        let _ = world.run_system_once(game_outcome_system);

        assert_eq!(*world.resource::<GamePhase>(), GamePhase::Playing);
    }
}
