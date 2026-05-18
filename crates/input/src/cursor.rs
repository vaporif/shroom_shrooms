use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

/// Swap the window cursor icon to signal which mode the left click is in:
/// a crosshair while `WispMode` is held, the default pointer otherwise.
///
/// Only writes `CursorIcon` when it actually changes, to avoid churning
/// change-detection and queuing a needless command every frame.
pub fn cursor_system(
    mut commands: Commands,
    actions: Res<ActionState<Action>>,
    window: Query<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
) {
    let Ok((window, current)) = window.single() else {
        return;
    };
    let icon = if actions.pressed(&Action::WispMode) {
        SystemCursorIcon::Crosshair
    } else {
        SystemCursorIcon::Default
    };
    let next = CursorIcon::System(icon);
    if current != Some(&next) {
        commands.entity(window).insert(next);
    }
}
