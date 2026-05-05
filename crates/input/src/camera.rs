use bevy::prelude::*;
use fungai_core::{Hex, HexLayout, HexOrientation, OffsetHexMode};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

#[derive(Component, Debug)]
pub struct GameCamera;

const CAMERA_SPEED: f32 = 300.0;
const ZOOM_SPEED: f32 = 0.1;
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
            ortho.scale = (ortho.scale - zoom_delta * ZOOM_SPEED).clamp(MIN_ZOOM, MAX_ZOOM);
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
}
