use std::collections::HashMap;

use bevy::prelude::*;

use crate::grid::Hex;

#[derive(Resource)]
pub struct TickTimer {
    pub timer: Timer,
}

impl Default for TickTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationSpeed {
    Paused,
    #[default]
    Normal,
    Fast,
    Fastest,
}

impl SimulationSpeed {
    #[must_use]
    pub fn duration_secs(self) -> f32 {
        match self {
            Self::Paused => 1.0,
            Self::Normal => 1.0,
            Self::Fast => 0.5,
            Self::Fastest => 0.25,
        }
    }

    #[must_use]
    pub fn is_paused(self) -> bool {
        matches!(self, Self::Paused)
    }

    #[must_use]
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Paused => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::Fastest,
            Self::Fastest => Self::Paused,
        }
    }

    #[must_use]
    pub fn speed_up(self) -> Self {
        match self {
            Self::Paused => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast | Self::Fastest => Self::Fastest,
        }
    }

    #[must_use]
    pub fn slow_down(self) -> Self {
        match self {
            Self::Paused | Self::Normal => Self::Paused,
            Self::Fast => Self::Normal,
            Self::Fastest => Self::Fast,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Paused => "\u{23f8} Paused",
            Self::Normal => "\u{25b6} 1x",
            Self::Fast => "\u{25b6}\u{25b6} 2x",
            Self::Fastest => "\u{25b6}\u{25b6}\u{25b6} 4x",
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationSet;

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GamePhase {
    #[default]
    Title,
    Playing,
    Victory,
    Defeat,
    Restarting,
}

#[derive(Resource, Default, Debug, Reflect)]
pub struct GameState {
    pub turn: u32,
    pub paused: bool,
    pub fragments_total: u32,
    pub fragments_fused: u32,
    pub mushrooms_fruited: u32,
    pub mushrooms_required: u32,
}

impl GameState {
    pub fn victory(&self) -> bool {
        self.fragments_fused >= self.fragments_total
            && self.mushrooms_fruited >= self.mushrooms_required
            && self.fragments_total > 0
    }
}

#[derive(Resource, Default, Debug)]
pub struct TerrainSpriteMap {
    pub sprites: HashMap<Hex, Entity>,
}

#[derive(Resource, Debug, Reflect)]
pub struct HintsVisible(pub bool);

impl Default for HintsVisible {
    fn default() -> Self {
        Self(true)
    }
}

pub fn tick_advancement_system(
    time: Res<Time>,
    mut tick_timer: ResMut<TickTimer>,
    mut game_state: ResMut<GameState>,
    speed: Res<SimulationSpeed>,
) {
    if speed.is_paused() {
        tick_timer.timer.pause();
    } else {
        tick_timer.timer.unpause();
        let target = std::time::Duration::from_secs_f32(speed.duration_secs());
        if tick_timer.timer.duration() != target {
            tick_timer.timer.set_duration(target);
        }
    }
    // Tick unconditionally: on a paused timer this is Bevy's documented state-reset
    // no-op (clears times_finished_this_tick) and prevents a stale just_finished from
    // leaking into the pause frame and triggering a phantom SimulationSet run.
    tick_timer.timer.tick(time.delta());
    if tick_timer.timer.just_finished() {
        game_state.turn += 1;
    }
}

fn simulation_should_tick(tick_timer: Res<TickTimer>) -> bool {
    tick_timer.timer.just_finished()
}

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, SimulationSet.run_if(simulation_should_tick))
            .add_systems(Update, tick_advancement_system.before(SimulationSet));
    }
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
        app.world_mut()
            .resource_mut::<TickTimer>()
            .timer
            .almost_finish();
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

    #[test]
    fn just_finished_does_not_leak_into_paused_frames() {
        let mut app = test_app();
        app.world_mut()
            .resource_mut::<TickTimer>()
            .timer
            .almost_finish();
        app.update();
        app.update();
        assert!(
            app.world().resource::<TickTimer>().timer.just_finished(),
            "precondition: the timer should have fired this frame",
        );

        app.insert_resource(SimulationSpeed::Paused);
        app.update();

        assert!(
            !app.world().resource::<TickTimer>().timer.just_finished(),
            "a paused frame must clear just_finished, otherwise SimulationSet runs a phantom tick",
        );
    }
}
