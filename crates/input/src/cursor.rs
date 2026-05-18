use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

/// Swap the window cursor icon to signal which mode the left click is in:
/// a crosshair while `WispMode` is held, the default pointer otherwise.
pub fn cursor_system(
    mut commands: Commands,
    actions: Res<ActionState<Action>>,
    window: Query<Entity, With<PrimaryWindow>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let icon = if actions.pressed(&Action::WispMode) {
        SystemCursorIcon::Crosshair
    } else {
        SystemCursorIcon::Default
    };
    commands.entity(window).insert(CursorIcon::System(icon));
}
