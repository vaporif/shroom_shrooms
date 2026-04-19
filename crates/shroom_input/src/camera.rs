use bevy::prelude::*;

use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseWheel;

#[derive(Component, Debug)]
pub struct GameCamera;

const CAMERA_SPEED: f32 = 300.0;
const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 0.15;
const MAX_ZOOM: f32 = 4.0;

pub fn spawn_camera(mut commands: Commands) {
    // Center on map midpoint (where the player starts)
    commands.spawn((
        Camera2d,
        GameCamera,
        Transform::from_xyz(1920.0, 1440.0, 0.0),
    ));
}

pub fn camera_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut query: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
) {
    let Ok((mut transform, mut projection)) = query.single_mut() else {
        return;
    };
    let delta = time.delta_secs();

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction.length_squared() > 0.0 {
        transform.translation += (direction.normalize() * CAMERA_SPEED * delta).extend(0.0);
    }

    if let Projection::Orthographic(ref mut ortho) = *projection {
        for event in scroll_events.read() {
            ortho.scale = (ortho.scale - event.y * ZOOM_SPEED).clamp(MIN_ZOOM, MAX_ZOOM);
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
