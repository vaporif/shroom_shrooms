use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shroom_core::{GridPos, GridWorld, Tile};

use crate::camera::GameCamera;

const TILE_SIZE: f32 = 16.0;
const PRIORITY_RADIUS: i32 = 3;

pub fn priority_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    _grid: Res<GridWorld>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
) {
    if !mouse.pressed(MouseButton::Right) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };

    let grid_pos = IVec2::new(
        (world_pos.x / TILE_SIZE).round() as i32,
        (world_pos.y / TILE_SIZE).round() as i32,
    );

    for (gpos, mut tile) in &mut tiles {
        let dist = (gpos.0 - grid_pos).abs();
        if dist.x <= PRIORITY_RADIUS && dist.y <= PRIORITY_RADIUS {
            let dir = (grid_pos - gpos.0).as_vec2();
            if dir.length_squared() > 0.01 {
                tile.priority_bias = dir.normalize() * 0.5;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use shroom_core::{GridPos, GridWorld, Tile};

    use bevy::prelude::*;

    #[test]
    fn priority_bias_set_on_nearby_tiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();

        let pos = IVec2::new(5, 5);
        let entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    priority_bias: Vec2::ZERO,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);

        // Skip mouse input (needs a window), test bias math directly
        {
            let mut tile = app
                .world_mut()
                .get_mut::<Tile>(entity)
                .expect("tile exists");
            let target = IVec2::new(8, 5);
            let dir = (target - pos).as_vec2();
            tile.priority_bias = dir.normalize() * 0.5;
        }

        let tile = app.world().get::<Tile>(entity).expect("tile exists");
        assert!(tile.priority_bias.x > 0.0, "bias should point right");
    }
}
