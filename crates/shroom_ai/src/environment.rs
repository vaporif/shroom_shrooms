use bevy::prelude::*;
use rand::prelude::*;
use rand::rngs::StdRng;
use shroom_core::*;

#[derive(Resource)]
pub struct EnvironmentRng(pub StdRng);

impl Default for EnvironmentRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(31))
    }
}

pub fn environment_threat_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    grid: Res<GridWorld>,
    mut rng: ResMut<EnvironmentRng>,
) {
    if game_state.turn.is_multiple_of(45) {
        let x = rng.0.random_range(0..grid.width);
        let y = grid.height - 1;
        let pos = IVec2::new(x, y);
        if grid.tiles.contains_key(&pos) {
            commands.spawn((
                GridPos(pos),
                FaunaAgent {
                    health: 3.0,
                    damage_per_tick: 0.2,
                },
            ));
        }
    }
}
