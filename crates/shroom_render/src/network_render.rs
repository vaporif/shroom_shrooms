use bevy::prelude::*;

use crate::data_layer::BranchGraph;

const TILE_SIZE: f32 = 16.0;

#[derive(Component)]
pub struct NetworkPathSprite;

/// Catmull-Rom spline segment: curve passes through p1..p2, with p0/p3 as tangent guides.
#[must_use]
pub fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Placeholder: sprite-based edge rendering. Will be replaced with spline meshes.
pub fn network_render_system(
    mut commands: Commands,
    graph: Res<BranchGraph>,
    existing: Query<Entity, With<NetworkPathSprite>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    for edge in &graph.edges {
        let from = edge.from.as_vec2() * TILE_SIZE;
        let to = edge.to.as_vec2() * TILE_SIZE;
        let mid = (from + to) * 0.5;

        let width = (edge.thickness * 2.0).clamp(2.0, 8.0);

        commands.spawn((
            NetworkPathSprite,
            Sprite {
                color: Color::srgb(0.9, 0.85, 0.7),
                custom_size: Some(Vec2::new(width, TILE_SIZE)),
                ..default()
            },
            Transform::from_translation(mid.extend(1.0)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catmull_rom_passes_through_control_points() {
        let p0 = Vec2::new(0.0, 0.0);
        let p1 = Vec2::new(1.0, 0.0);
        let p2 = Vec2::new(2.0, 1.0);
        let p3 = Vec2::new(3.0, 1.0);

        let at_start = catmull_rom(p0, p1, p2, p3, 0.0);
        let at_end = catmull_rom(p0, p1, p2, p3, 1.0);

        assert!((at_start - p1).length() < 0.001);
        assert!((at_end - p2).length() < 0.001);
    }

    #[test]
    fn catmull_rom_midpoint_is_between_control_points() {
        let p0 = Vec2::ZERO;
        let p1 = Vec2::new(1.0, 0.0);
        let p2 = Vec2::new(2.0, 0.0);
        let p3 = Vec2::new(3.0, 0.0);

        let mid = catmull_rom(p0, p1, p2, p3, 0.5);

        assert!((mid - Vec2::new(1.5, 0.0)).length() < 0.001);
    }
}
