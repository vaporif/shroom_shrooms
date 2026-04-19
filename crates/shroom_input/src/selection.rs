use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shroom_core::*;

use crate::camera::GameCamera;

const TILE_SIZE: f32 = 16.0;

#[derive(Resource, Default)]
pub struct SelectedRegion {
    pub region_id: Option<RegionId>,
    pub selected_pos: Option<IVec2>,
}

pub fn selection_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut selected: ResMut<SelectedRegion>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
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

    if let Some(&entity) = grid.tiles.get(&grid_pos) {
        if let Ok(tile) = tiles.get(entity) {
            selected.selected_pos = Some(grid_pos);
            selected.region_id = tile.occupant.region_id();
        }
    }
}
