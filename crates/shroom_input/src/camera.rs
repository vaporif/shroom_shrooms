use bevy::prelude::*;

use bevy::ecs::message::MessageReader;
use bevy::input::mouse::MouseWheel;

#[derive(Component, Debug)]
pub struct GameCamera;

const CAMERA_SPEED: f32 = 300.0;
const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 3.0;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, GameCamera, Transform::from_xyz(640.0, 480.0, 0.0)));
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
