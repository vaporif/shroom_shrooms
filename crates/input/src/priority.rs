use bevy::prelude::*;
use fungai_core::{GridPos, HexLayout, SelectedRegion, Tile};

const PRIORITY_RADIUS: i32 = 3;

pub fn priority_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedRegion>,
    layout: Res<HexLayout>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
) {
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if keyboard.just_pressed(KeyCode::KeyP) && shift {
        for (_gpos, mut tile) in &mut tiles {
            tile.priority_bias = Vec2::ZERO;
        }
        return;
    }

    if !keyboard.just_pressed(KeyCode::KeyP) {
        return;
    }

    let Some(target_hex) = selected.selected_pos else {
        return;
    };

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
    use super::*;
    use fungai_core::{create_hex_layout, GridPos, GridWorld, Hex, Tile};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
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
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
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

        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(KeyCode::ShiftLeft);
        input.press(KeyCode::KeyP);
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

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::new(0.5, 0.0));
    }
}
