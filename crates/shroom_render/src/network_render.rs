use bevy::asset::RenderAssetUsages;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;

use crate::data_layer::BranchGraph;

const TILE_SIZE: f32 = 16.0;
const SPLINE_SAMPLES: usize = 8;

#[derive(Component)]
pub struct NetworkPathSprite;

#[derive(Component)]
pub struct NetworkMesh;

#[derive(Component)]
pub struct JunctionMesh;

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

/// Map a specialization to its display color.
#[must_use]
fn region_color(spec: Option<shroom_core::SpecializationType>) -> Color {
    use shroom_core::SpecializationType;
    match spec {
        Some(SpecializationType::Explorer) => Color::srgb(1.0, 0.9, 0.3),
        Some(SpecializationType::Parasite) => Color::srgb(0.8, 0.2, 0.2),
        Some(SpecializationType::Researcher) => Color::srgb(0.3, 0.5, 0.9),
        Some(SpecializationType::Hunter) => Color::srgb(0.6, 0.4, 0.1),
        Some(SpecializationType::Decomposer) => Color::srgb(0.2, 0.7, 0.3),
        Some(SpecializationType::Symbiont) => Color::srgb(0.3, 0.8, 0.8),
        Some(SpecializationType::Infiltrator) => Color::srgb(0.6, 0.3, 0.8),
        Some(SpecializationType::Transporter) => Color::srgb(0.9, 0.6, 0.2),
        None => Color::srgb(0.9, 0.85, 0.7),
    }
}

/// Build a triangle-strip mesh for a Catmull-Rom spline between two endpoints.
///
/// Returns the mesh and the list of sampled centerline points (useful for testing).
#[must_use]
fn build_spline_mesh(from: Vec2, to: Vec2, half_width: f32) -> (Mesh, Vec<Vec2>) {
    // Extrapolate control points for tangent continuity
    let dir = to - from;
    let p0 = from - dir;
    let p3 = to + dir;

    // Sample centerline
    let mut points = Vec::with_capacity(SPLINE_SAMPLES);
    for i in 0..SPLINE_SAMPLES {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / (SPLINE_SAMPLES - 1) as f32;
        points.push(catmull_rom(p0, from, to, p3, t));
    }

    // Build triangle strip vertices
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(SPLINE_SAMPLES * 2);

    for i in 0..SPLINE_SAMPLES {
        // Compute tangent via finite differences
        let tangent = if i == 0 {
            points[1] - points[0]
        } else if i == SPLINE_SAMPLES - 1 {
            points[SPLINE_SAMPLES - 1] - points[SPLINE_SAMPLES - 2]
        } else {
            points[i + 1] - points[i - 1]
        };

        let normal = Vec2::new(-tangent.y, tangent.x).normalize_or_zero();
        let left = points[i] + normal * half_width;
        let right = points[i] - normal * half_width;

        positions.push([left.x, left.y, 0.0]);
        positions.push([right.x, right.y, 0.0]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleStrip,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    (mesh, points)
}

/// Count how many edges connect to each node for junction detection.
fn count_node_edges(graph: &BranchGraph) -> std::collections::HashMap<IVec2, usize> {
    let mut counts: std::collections::HashMap<IVec2, usize> = std::collections::HashMap::new();
    for edge in &graph.edges {
        *counts.entry(edge.from).or_default() += 1;
        *counts.entry(edge.to).or_default() += 1;
    }
    counts
}

pub fn network_render_system(
    mut commands: Commands,
    graph: Res<BranchGraph>,
    existing_meshes: Query<Entity, With<NetworkMesh>>,
    existing_junctions: Query<Entity, With<JunctionMesh>>,
    existing_sprites: Query<Entity, With<NetworkPathSprite>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Despawn all previous visuals
    for entity in existing_meshes.iter() {
        commands.entity(entity).despawn();
    }
    for entity in existing_junctions.iter() {
        commands.entity(entity).despawn();
    }
    for entity in existing_sprites.iter() {
        commands.entity(entity).despawn();
    }

    // Render each edge as a spline mesh
    for edge in &graph.edges {
        let from = edge.from.as_vec2() * TILE_SIZE;
        let to = edge.to.as_vec2() * TILE_SIZE;
        let width = (edge.thickness * 2.0).clamp(2.0, 8.0);
        let half_width = width * 0.5;

        // Determine color from the "from" node's region specialization
        let spec = graph.nodes.get(&edge.from).and_then(|n| n.specialization);
        let color = region_color(spec);

        let (mesh, _points) = build_spline_mesh(from, to, half_width);

        commands.spawn((
            NetworkMesh,
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        ));
    }

    // Junction circles at branching nodes (3+ edges)
    let edge_counts = count_node_edges(&graph);
    for (&pos, &count) in &edge_counts {
        if count >= 3 {
            let spec = graph.nodes.get(&pos).and_then(|n| n.specialization);
            let color = region_color(spec);
            let world_pos = pos.as_vec2() * TILE_SIZE;

            commands.spawn((
                JunctionMesh,
                Mesh2d(meshes.add(Circle::new(4.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
                Transform::from_translation(world_pos.extend(1.5)),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::SpecializationType;

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

    #[test]
    fn spline_mesh_has_correct_vertex_count() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _points) = build_spline_mesh(from, to, 2.0);

        // 8 samples, 2 vertices each (left + right) = 16 vertices
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("mesh should have positions");

        let count = match positions {
            bevy::mesh::VertexAttributeValues::Float32x3(v) => v.len(),
            _ => panic!("unexpected attribute format"),
        };

        assert_eq!(count, SPLINE_SAMPLES * 2);
    }

    #[test]
    fn spline_mesh_vertices_surround_centerline() {
        let from = Vec2::new(10.0, 20.0);
        let to = Vec2::new(110.0, 20.0);
        let half_width = 3.0;
        let (mesh, _points) = build_spline_mesh(from, to, half_width);

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("mesh should have positions");

        let verts = match positions {
            bevy::mesh::VertexAttributeValues::Float32x3(v) => v,
            _ => panic!("unexpected attribute format"),
        };

        // First pair should straddle the from point
        let left = Vec2::new(verts[0][0], verts[0][1]);
        let right = Vec2::new(verts[1][0], verts[1][1]);
        let midpoint = (left + right) * 0.5;

        // Midpoint of first vertex pair should be close to the from point
        assert!(
            (midpoint - from).length() < 0.01,
            "midpoint {midpoint} should be near from {from}"
        );

        // The two vertices should be separated by approximately 2 * half_width
        let separation = (left - right).length();
        assert!(
            (separation - half_width * 2.0).abs() < 0.01,
            "separation {separation} should be near {half_width_2}",
            half_width_2 = half_width * 2.0
        );
    }

    #[test]
    fn region_color_maps_specializations() {
        // Spot-check a few specialization colors
        let explorer = region_color(Some(SpecializationType::Explorer));
        assert_eq!(explorer, Color::srgb(1.0, 0.9, 0.3));

        let parasite = region_color(Some(SpecializationType::Parasite));
        assert_eq!(parasite, Color::srgb(0.8, 0.2, 0.2));

        let none_color = region_color(None);
        assert_eq!(none_color, Color::srgb(0.9, 0.85, 0.7));

        let hunter = region_color(Some(SpecializationType::Hunter));
        assert_eq!(hunter, Color::srgb(0.6, 0.4, 0.1));
    }
}
