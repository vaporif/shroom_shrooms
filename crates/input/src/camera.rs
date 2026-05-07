use bevy::prelude::*;
use kingdom_core::{Hex, HexLayout, HexOrientation, OffsetHexMode};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

#[derive(Component, Debug)]
pub struct GameCamera;

const CAMERA_SPEED: f32 = 300.0;
const ZOOM_FACTOR_PER_TICK: f32 = 1.15;
const MIN_ZOOM: f32 = 0.15;
const MAX_ZOOM: f32 = 4.0;

pub fn spawn_camera(mut commands: Commands, layout: Res<HexLayout>) {
    let center_hex =
        Hex::from_offset_coordinates([40, 30], OffsetHexMode::Odd, HexOrientation::Pointy);
    let center = layout.hex_to_world_pos(center_hex);
    commands.spawn((
        Camera2d,
        GameCamera,
        Transform::from_xyz(center.x, center.y, 0.0),
    ));
}

pub fn camera_system(
    time: Res<Time>,
    actions: Res<ActionState<Action>>,
    mut query: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
) {
    let Ok((mut transform, mut projection)) = query.single_mut() else {
        return;
    };
    let delta = time.delta_secs();

    let direction = actions.axis_pair(&Action::CameraMove);
    if direction.length_squared() > 0.0 {
        transform.translation += (direction.normalize() * CAMERA_SPEED * delta).extend(0.0);
    }

    if let Projection::Orthographic(ref mut ortho) = *projection {
        let zoom_delta = actions.value(&Action::Zoom);
        if zoom_delta != 0.0 {
            // Positive scroll → zoom in (smaller scale); use ZOOM_FACTOR^(-delta)
            // so each tick is a uniform visual ratio change.
            let factor = ZOOM_FACTOR_PER_TICK.powf(-zoom_delta);
            ortho.scale = (ortho.scale * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_range_matches_spec() {
        assert_eq!(MIN_ZOOM, 0.15);
        assert_eq!(MAX_ZOOM, 4.0);
    }

    #[test]
    fn zoom_factor_is_multiplicative_uniform() {
        // One scroll tick should scale by ZOOM_FACTOR_PER_TICK regardless of
        // current scale, so a constant-input log range fully traverses [MIN, MAX]
        // in a small, uniform number of ticks.
        let ticks_to_traverse = (MAX_ZOOM / MIN_ZOOM).ln() / ZOOM_FACTOR_PER_TICK.ln();
        assert!(
            ticks_to_traverse > 15.0 && ticks_to_traverse < 30.0,
            "expected ~22 ticks to traverse range, got {ticks_to_traverse}"
        );
    }
}
