use std::time::Duration;

use bevy::prelude::*;
use shroom_core::{SimulationSpeed, TickTimer};

pub fn speed_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut speed: ResMut<SimulationSpeed>,
    mut tick_timer: ResMut<TickTimer>,
) {
    let mut changed = false;

    if keyboard.just_pressed(KeyCode::Space) {
        *speed = if speed.is_paused() {
            SimulationSpeed::Normal
        } else {
            SimulationSpeed::Paused
        };
        changed = true;
    }

    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        *speed = speed.speed_up();
        changed = true;
    }

    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        *speed = speed.slow_down();
        changed = true;
    }

    if changed && !speed.is_paused() {
        tick_timer
            .timer
            .set_duration(Duration::from_secs_f32(speed.duration_secs()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_app(initial_speed: SimulationSpeed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(initial_speed);
        app.insert_resource(TickTimer::default());
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_systems(Update, speed_input_system);
        app
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
    }

    #[test]
    fn space_toggles_pause() {
        let mut app = setup_app(SimulationSpeed::Normal);
        press_key(&mut app, KeyCode::Space);
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Paused
        );
    }

    #[test]
    fn plus_speeds_up() {
        let mut app = setup_app(SimulationSpeed::Normal);
        press_key(&mut app, KeyCode::Equal);
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Fast
        );
    }

    #[test]
    fn minus_slows_down() {
        let mut app = setup_app(SimulationSpeed::Fast);
        press_key(&mut app, KeyCode::Minus);
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Normal
        );
    }
}
