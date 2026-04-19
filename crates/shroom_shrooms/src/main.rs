use bevy::prelude::*;

use shroom_ai::AiPlugin;
use shroom_core::{CorePlugin, GameState, SimulationSet, SimulationSpeed, TickTimer};
use shroom_fruiting::FruitingPlugin;
use shroom_growth::GrowthPlugin;
use shroom_input::InputPlugin;
use shroom_regions::RegionsPlugin;
use shroom_render::RenderPlugin;
use shroom_ui::UiPlugin;
use shroom_world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            CorePlugin,
            WorldPlugin,
            GrowthPlugin,
            RegionsPlugin,
            RenderPlugin,
            InputPlugin,
            AiPlugin,
            FruitingPlugin,
            UiPlugin,
        ))
        .configure_sets(Update, SimulationSet.run_if(simulation_should_tick))
        .add_systems(Update, tick_advancement_system.before(SimulationSet))
        .run();
}

pub fn tick_advancement_system(
    time: Res<Time>,
    mut tick_timer: ResMut<TickTimer>,
    mut game_state: ResMut<GameState>,
    speed: Res<SimulationSpeed>,
) {
    if speed.is_paused() {
        tick_timer.timer.pause();
        return;
    }
    tick_timer.timer.unpause();
    let target = std::time::Duration::from_secs_f32(speed.duration_secs());
    if tick_timer.timer.duration() != target {
        tick_timer.timer.set_duration(target);
    }
    tick_timer.timer.tick(time.delta());
    if tick_timer.timer.just_finished() {
        game_state.turn += 1;
    }
}

fn simulation_should_tick(tick_timer: Res<TickTimer>) -> bool {
    tick_timer.timer.just_finished()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<TickTimer>();
        app.init_resource::<GameState>();
        app.init_resource::<SimulationSpeed>();
        app.add_systems(Update, tick_advancement_system);
        app
    }

    #[test]
    fn tick_advances_turn_after_timer_completes() {
        let mut app = test_app();
        // Set timer to almost finished so even a tiny delta will complete it
        app.world_mut()
            .resource_mut::<TickTimer>()
            .timer
            .almost_finish();
        // First update primes the time source, second update gives a real delta
        app.update();
        app.update();
        let gs = app.world().resource::<GameState>();
        assert!(gs.turn >= 1, "turn should advance after timer completes");
    }

    #[test]
    fn tick_does_not_advance_when_paused() {
        let mut app = test_app();
        app.insert_resource(SimulationSpeed::Paused);
        app.world_mut().resource_mut::<TickTimer>().timer =
            Timer::from_seconds(0.001, TimerMode::Repeating);
        app.update();
        let gs = app.world().resource::<GameState>();
        assert_eq!(gs.turn, 0, "turn should not advance when paused");
    }
}
