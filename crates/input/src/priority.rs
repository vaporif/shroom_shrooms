use bevy::prelude::*;
use fungai_core::{GridPos, HexLayout, SelectedRegion, Tile};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

const PRIORITY_RADIUS: i32 = 3;

pub fn priority_system(
    actions: Res<ActionState<Action>>,
    selected: Res<SelectedRegion>,
    layout: Res<HexLayout>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
) {
    // ClearPriority is the longer chord; with PrioritizeLongest clash strategy
    // it suppresses SetPriority. Check it first as a defensive guard against
    // strategy changes.
    if actions.just_pressed(&Action::ClearPriority) {
        for (_gpos, mut tile) in &mut tiles {
            tile.priority_bias = Vec2::ZERO;
        }
        return;
    }

    if !actions.just_pressed(&Action::SetPriority) {
        return;
    }

    let Some(target_hex) = selected.selected_pos else {
        return;
    };

    for (gpos, mut tile) in &mut tiles {
        let dist = gpos.0.distance_to(target_hex);
        tile.priority_bias = if dist <= PRIORITY_RADIUS {
            let dir = layout.hex_to_world_pos(target_hex) - layout.hex_to_world_pos(gpos.0);
            if dir.length_squared() > 0.01 {
                dir.normalize() * 0.5
            } else {
                Vec2::ZERO
            }
        } else {
            Vec2::ZERO
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin as BevyInputPlugin;
    use fungai_core::{GridPos, GridWorld, Hex, Tile, create_hex_layout};

    use crate::action::{Action, default_input_map};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BevyInputPlugin);
        app.add_plugins(InputManagerPlugin::<Action>::default());
        app.insert_resource(default_input_map());
        app.init_resource::<ActionState<Action>>();
        app.init_resource::<GridWorld>();
        app.init_resource::<SelectedRegion>();
        app.insert_resource(create_hex_layout());
        app.add_systems(Update, priority_system);
        app
    }

    fn spawn_tile(app: &mut App, hex: Hex) -> Entity {
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
        entity
    }

    #[test]
    fn p_key_sets_bias_around_selected_tile() {
        let mut app = test_app();
        let target = Hex::new(8, -3);
        let near = Hex::new(5, -3);
        let _ = spawn_tile(&mut app, target);
        let near_entity = spawn_tile(&mut app, near);

        app.world_mut()
            .resource_mut::<SelectedRegion>()
            .selected_pos = Some(target);

        KeyCode::KeyP.press(app.world_mut());
        app.update();

        let tile = app.world().get::<Tile>(near_entity).expect("tile exists");
        assert!(
            tile.priority_bias.length_squared() > 0.0,
            "near tile should have bias"
        );
    }

    #[test]
    fn shift_p_clears_bias() {
        let mut app = test_app();
        let hex = Hex::new(0, 0);
        let entity = spawn_tile(&mut app, hex);
        app.world_mut()
            .get_mut::<Tile>(entity)
            .unwrap()
            .priority_bias = Vec2::new(0.5, 0.0);

        KeyCode::ShiftLeft.press(app.world_mut());
        KeyCode::KeyP.press(app.world_mut());
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::ZERO);
    }

    #[test]
    fn p_with_no_selection_is_noop() {
        let mut app = test_app();
        let hex = Hex::new(0, 0);
        let entity = spawn_tile(&mut app, hex);
        app.world_mut()
            .get_mut::<Tile>(entity)
            .unwrap()
            .priority_bias = Vec2::new(0.5, 0.0);

        KeyCode::KeyP.press(app.world_mut());
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::new(0.5, 0.0));
    }
}
