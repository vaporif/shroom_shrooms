use bevy::ecs::message::MessageWriter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    GamePhase, GridPos, GridWorld, Hex, HexLayout, SelectedUnit, Tile, Unit, UnitMovement,
};
use kingdom_units::find_path;
use leafwing_input_manager::prelude::*;

use crate::action::Action;
use crate::camera::GameCamera;
use crate::wisp::TileTapped;

/// First unit entity occupying `hex`, if any.
pub fn unit_at<'a>(
    hex: Hex,
    units: impl Iterator<Item = (Entity, &'a GridPos, &'a Unit)>,
) -> Option<Entity> {
    units
        .into_iter()
        .find(|(_, gp, _)| gp.0 == hex)
        .map(|(e, _, _)| e)
}

#[derive(SystemParam)]
pub struct PointerInput<'w, 's> {
    actions: Res<'w, ActionState<Action>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
    ui_interactions: Query<'w, 's, &'static Interaction, With<Button>>,
}

impl PointerInput<'_, '_> {
    fn cursor_hex(&self, layout: &HexLayout) -> Option<Hex> {
        let window = self.windows.single().ok()?;
        let cursor = window.cursor_position()?;
        let (camera, cam_xform) = self.cameras.single().ok()?;
        let world = camera.viewport_to_world_2d(cam_xform, cursor).ok()?;
        Some(layout.world_pos_to_hex(world))
    }

    fn ui_blocking(&self) -> bool {
        self.ui_interactions
            .iter()
            .any(|i| !matches!(i, Interaction::None))
    }
}

#[expect(clippy::too_many_arguments)]
pub fn pointer_system(
    input: PointerInput,
    phase: Res<GamePhase>,
    layout: Res<HexLayout>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    units: Query<(Entity, &GridPos, &Unit)>,
    mut movements: Query<&mut UnitMovement>,
    mut selected: ResMut<SelectedUnit>,
    mut taps: MessageWriter<TileTapped>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    // The wisp owns the click while WispMode is held.
    if input.actions.pressed(&Action::WispMode) {
        return;
    }
    if !input.actions.just_pressed(&Action::Paint) || input.ui_blocking() {
        return;
    }
    let Some(hex) = input.cursor_hex(&layout) else {
        return;
    };

    if let Some(unit) = unit_at(hex, units.iter()) {
        selected.0 = Some(unit);
        return;
    }

    if let Some(unit) = selected.0
        && let Ok((_, start, _)) = units.get(unit)
    {
        let path = find_path(start.0, hex, &grid, |h| {
            grid.tiles
                .get(&h)
                .and_then(|&e| tiles.get(e).ok())
                .is_some_and(|t| t.terrain.is_passable())
        });
        if let Some(path) = path
            && let Ok(mut movement) = movements.get_mut(unit)
        {
            movement.path = path;
            movement.edge_progress = 0.0;
        }
        return;
    }

    selected.0 = None;
    taps.write(TileTapped { pos: hex });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexx::Hex;
    use kingdom_core::{RegionId, UnitKind};

    #[test]
    fn click_on_unit_hex_finds_that_unit() {
        let mut world = World::new();
        let unit = world
            .spawn((
                GridPos(Hex::new(2, 3)),
                Unit {
                    kind: UnitKind::Founder,
                    owner: RegionId(0),
                },
            ))
            .id();
        let mut q = world.query::<(Entity, &GridPos, &Unit)>();
        let found = unit_at(Hex::new(2, 3), q.iter(&world));
        assert_eq!(found, Some(unit));
        let mut q2 = world.query::<(Entity, &GridPos, &Unit)>();
        assert_eq!(unit_at(Hex::new(9, 9), q2.iter(&world)), None);
    }
}
