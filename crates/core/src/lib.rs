mod components;
mod config;
mod constants;
mod grid;
mod messages;
mod region;
mod simulation;
mod tile;
mod unlock;

pub use components::*;
pub use config::*;
pub use constants::*;
pub use grid::*;
pub use messages::*;
pub use region::*;
pub use simulation::*;
pub use tile::*;
pub use unlock::*;

use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(create_hex_layout())
            .init_resource::<LaunchConfig>()
            .init_resource::<GridWorld>()
            .init_resource::<RegionStates>()
            .init_resource::<GameState>()
            .init_resource::<TickTimer>()
            .init_resource::<SimulationSpeed>()
            .init_resource::<GamePhase>()
            .init_resource::<SelectedRegion>()
            .add_message::<TurnAdvanced>()
            .add_message::<TileDiscovered>()
            .add_message::<DecompositionComplete>()
            .add_message::<FragmentFused>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_timer_defaults_to_one_second_repeating() {
        let tick = TickTimer::default();
        assert_eq!(tick.timer.duration().as_secs_f32(), 1.0);
        assert!(tick.timer.mode() == TimerMode::Repeating);
    }

    #[test]
    fn simulation_speed_duration() {
        assert_eq!(SimulationSpeed::Normal.duration_secs(), 1.0);
        assert_eq!(SimulationSpeed::Fast.duration_secs(), 0.5);
        assert_eq!(SimulationSpeed::Fastest.duration_secs(), 0.25);
    }

    #[test]
    fn simulation_speed_is_paused() {
        assert!(SimulationSpeed::Paused.is_paused());
        assert!(!SimulationSpeed::Normal.is_paused());
    }

    #[test]
    fn grid_pos_wraps_hex() {
        let h = Hex::new(3, -2);
        let gp = GridPos(h);
        assert_eq!(gp.0, h);
        assert_eq!(gp.0.x, 3);
        assert_eq!(gp.0.y, -2);
    }

    #[test]
    fn grid_world_neighbors_returns_six_when_all_exist() {
        let mut world = GridWorld::default();
        let center = Hex::ZERO;

        world.tiles.insert(center, Entity::from_bits(1));
        for (i, neighbor) in center.all_neighbors().into_iter().enumerate() {
            world
                .tiles
                .insert(neighbor, Entity::from_bits((i + 2) as u64));
        }

        let neighbors: Vec<_> = world.neighbors(center).collect();
        assert_eq!(neighbors.len(), 6);

        for (pos, _entity) in &neighbors {
            assert!(center.all_neighbors().contains(pos));
        }
    }

    #[test]
    fn grid_world_neighbors_excludes_missing_tiles() {
        let mut world = GridWorld::default();
        let center = Hex::ZERO;
        world.tiles.insert(center, Entity::from_bits(1));

        let all = center.all_neighbors();
        world.tiles.insert(all[0], Entity::from_bits(2));
        world.tiles.insert(all[3], Entity::from_bits(3));

        let neighbors: Vec<_> = world.neighbors(center).collect();
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn hex_layout_coordinate_round_trip() {
        let layout = create_hex_layout();
        let original = Hex::new(5, -3);
        let world_pos = layout.hex_to_world_pos(original);
        let recovered = layout.world_pos_to_hex(world_pos);
        assert_eq!(recovered, original);
    }
}
