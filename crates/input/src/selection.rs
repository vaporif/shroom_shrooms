use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use fungai_core::*;

use crate::camera::GameCamera;

#[allow(clippy::too_many_arguments)]
pub fn selection_system(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut selected: ResMut<SelectedRegion>,
    ui_interactions: Query<&Interaction, With<Button>>,
    layout: Res<HexLayout>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    // Don't process world clicks when UI buttons are being pressed
    for interaction in ui_interactions.iter() {
        if *interaction != Interaction::None {
            return;
        }
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

    let hex = layout.world_pos_to_hex(world_pos);

    let Some(&entity) = grid.tiles.get(&hex) else {
        return;
    };

    if let Ok(tile) = tiles.get(entity) {
        selected.selected_pos = Some(hex);
        selected.region_id = tile.occupant.region_id();
    }
}
