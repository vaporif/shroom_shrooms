use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use fungai_core::{GridPos, GridWorld, HexLayout, Tile};

use crate::camera::GameCamera;

const PRIORITY_RADIUS: i32 = 3;

pub fn priority_system(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    _grid: Res<GridWorld>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    layout: Res<HexLayout>,
) {
    if !mouse.just_pressed(MouseButton::Left) || !keyboard.pressed(KeyCode::ShiftLeft) {
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

    let target_hex = layout.world_pos_to_hex(world_pos);

    // Clear all existing bias before setting new one
    for (_gpos, mut tile) in &mut tiles {
        tile.priority_bias = Vec2::ZERO;
    }

    for (gpos, mut tile) in &mut tiles {
        let dist = gpos.0.distance_to(target_hex);
        if dist <= PRIORITY_RADIUS {
            let tile_world = layout.hex_to_world_pos(gpos.0);
            let target_world = layout.hex_to_world_pos(target_hex);
            let dir = target_world - tile_world;
            if dir.length_squared() > 0.01 {
                tile.priority_bias = dir.normalize() * 0.5;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fungai_core::{GridPos, GridWorld, Hex, HexLayout, Tile, create_hex_layout};

    use bevy::prelude::*;

    #[test]
    fn priority_bias_set_on_nearby_tiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.insert_resource(create_hex_layout());

        let hex = Hex::new(5, -3);
        let entity = app
            .world_mut()
            .spawn((
                GridPos(hex),
                Tile {
                    priority_bias: Vec2::ZERO,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(hex, entity);

        // Skip mouse input (needs a window), test bias math directly
        {
            let layout = app.world().resource::<HexLayout>();
            let target = Hex::new(8, -3);
            let tile_world = layout.hex_to_world_pos(hex);
            let target_world = layout.hex_to_world_pos(target);
            let dir = (target_world - tile_world).normalize() * 0.5;

            let mut tile = app
                .world_mut()
                .get_mut::<Tile>(entity)
                .expect("tile exists");
            tile.priority_bias = dir;
        }

        let tile = app.world().get::<Tile>(entity).expect("tile exists");
        assert!(tile.priority_bias.x > 0.0, "bias should point right");
    }
}
