use std::collections::{HashMap, HashSet, VecDeque};

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    mesh::PrimitiveTopology,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};
use fungai_core::{HexLayout, RegionId, RivalId};
use hexx::Hex;

use crate::data_layer::{BranchEdge, BranchGraph, RivalBranchGraph};

const SPLINE_SAMPLES: usize = 12;
const STRANDS_PER_EDGE: usize = 3;
const STRAND_HALF_WIDTH: f32 = 1.5;

#[derive(Component)]
pub struct BranchTreeMesh;

/// Packed uniform struct -- matches the WGSL `NetworkUniforms` struct exactly.
#[derive(ShaderType, Debug, Clone)]
pub struct NetworkUniforms {
    pub core_color: LinearRgba, // vec4<f32> -- 16 bytes
    pub body_color: LinearRgba, // vec4<f32> -- 16 bytes
    pub biomass: f32,           // f32 -- 4 bytes
    pub time: f32,              // f32 -- 4 bytes
    pub _padding: Vec2,         // pad to 16-byte boundary -- 8 bytes
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct NetworkMaterial {
    #[uniform(0)]
    pub uniforms: NetworkUniforms,
}

impl Material2d for NetworkMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/network.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

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

/// Map a specialization to its core color as `LinearRgba`.
#[must_use]
fn region_color_linear(spec: Option<fungai_core::SpecializationType>) -> LinearRgba {
    use fungai_core::SpecializationType;
    match spec {
        Some(SpecializationType::Explorer) => LinearRgba::new(1.0, 0.9, 0.3, 1.0),
        Some(SpecializationType::Parasite) => LinearRgba::new(0.8, 0.2, 0.2, 1.0),
        Some(SpecializationType::Researcher) => LinearRgba::new(0.3, 0.5, 0.9, 1.0),
        Some(SpecializationType::Hunter) => LinearRgba::new(0.6, 0.4, 0.1, 1.0),
        Some(SpecializationType::Decomposer) => LinearRgba::new(0.2, 0.7, 0.3, 1.0),
        Some(SpecializationType::Symbiont) => LinearRgba::new(0.3, 0.8, 0.8, 1.0),
        Some(SpecializationType::Infiltrator) => LinearRgba::new(0.6, 0.3, 0.8, 1.0),
        Some(SpecializationType::Transporter) => LinearRgba::new(0.9, 0.6, 0.2, 1.0),
        None => LinearRgba::new(0.9, 0.85, 0.7, 1.0),
    }
}

/// Derive a muted body color from a bright core color using luminance mixing.
#[must_use]
fn body_color_from_core(core: LinearRgba) -> LinearRgba {
    let gray = core.red * 0.299 + core.green * 0.587 + core.blue * 0.114;
    LinearRgba::new(
        (core.red * 0.4 + gray * 0.6) * 0.5,
        (core.green * 0.4 + gray * 0.6) * 0.5,
        (core.blue * 0.4 + gray * 0.6) * 0.5,
        0.7,
    )
}

/// Build a triangle-strip mesh for a Catmull-Rom spline between two endpoints.
///
/// Returns the mesh and the list of sampled centerline points (useful for testing).
/// UV_0: left vertex gets `[-1.0, v]`, right vertex gets `[1.0, v]` where v in [0, 1].
#[cfg(test)]
#[must_use]
fn build_spline_mesh(from: Vec2, to: Vec2, half_width: f32) -> (Mesh, Vec<Vec2>) {
    build_spline_mesh_inner(from, to, half_width, None)
}

/// Build a spline mesh with per-sample perpendicular noise wobble on interior points.
///
/// `seed` drives a deterministic hash so the same edge always produces the same shape.
/// Endpoints are never displaced to preserve junction alignment.
#[must_use]
fn build_spline_mesh_with_wobble(
    from: Vec2,
    to: Vec2,
    half_width: f32,
    seed: u32,
) -> (Mesh, Vec<Vec2>) {
    build_spline_mesh_inner(from, to, half_width, Some(seed))
}

/// Shared implementation -- `wobble_seed = None` for straight, `Some(seed)` for wobble.
fn build_spline_mesh_inner(
    from: Vec2,
    to: Vec2,
    half_width: f32,
    wobble_seed: Option<u32>,
) -> (Mesh, Vec<Vec2>) {
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

    // Apply perpendicular wobble to interior points when a seed is provided
    if let Some(seed) = wobble_seed {
        let branch_len = dir.length();
        let wobble_scale = (branch_len * 0.12).min(8.0);

        // Precompute tangents before mutating points
        let mut normals: Vec<Vec2> = Vec::with_capacity(SPLINE_SAMPLES);
        for i in 0..SPLINE_SAMPLES {
            let tangent = if i == 0 {
                points[1] - points[0]
            } else if i == SPLINE_SAMPLES - 1 {
                points[SPLINE_SAMPLES - 1] - points[SPLINE_SAMPLES - 2]
            } else {
                points[i + 1] - points[i - 1]
            };
            normals.push(Vec2::new(-tangent.y, tangent.x).normalize_or_zero());
        }

        for i in 1..(SPLINE_SAMPLES - 1) {
            #[allow(clippy::cast_possible_truncation)]
            let hash = seed
                .wrapping_mul(2_654_435_761)
                .wrapping_add(i as u32 * 73_856_093);
            #[allow(clippy::cast_precision_loss)]
            let noise = (hash as f32 / u32::MAX as f32) * 2.0 - 1.0;
            points[i] += normals[i] * noise * wobble_scale;
        }
    }

    // Build triangle-strip vertices + UV_0
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(SPLINE_SAMPLES * 2);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(SPLINE_SAMPLES * 2);

    for i in 0..SPLINE_SAMPLES {
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

        #[allow(clippy::cast_precision_loss)]
        let v = i as f32 / (SPLINE_SAMPLES - 1) as f32;
        uvs.push([-1.0, v]);
        uvs.push([1.0, v]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleStrip,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    (mesh, points)
}

/// Count how many edges connect to each node for junction/leaf detection.
#[cfg(test)]
fn compute_node_degrees(graph: &BranchGraph) -> HashMap<Hex, usize> {
    let mut counts: HashMap<Hex, usize> = HashMap::new();
    for edge in &graph.edges {
        *counts.entry(edge.from).or_default() += 1;
        *counts.entry(edge.to).or_default() += 1;
    }
    counts
}

/// Group player nodes by region, returning position and biomass pairs.
#[must_use]
fn group_player_nodes_by_region(graph: &BranchGraph) -> HashMap<RegionId, Vec<(Hex, f32)>> {
    let mut groups: HashMap<RegionId, Vec<(Hex, f32)>> = HashMap::new();
    for node in graph.nodes.values() {
        groups
            .entry(node.region_id)
            .or_default()
            .push((node.pos, node.biomass));
    }
    groups
}

/// Group rival nodes by rival_id, returning position and biomass pairs.
#[must_use]
fn group_rival_nodes_by_id(graph: &RivalBranchGraph) -> HashMap<RivalId, Vec<(Hex, f32)>> {
    let mut groups: HashMap<RivalId, Vec<(Hex, f32)>> = HashMap::new();
    for node in graph.nodes.values() {
        groups
            .entry(node.rival_id)
            .or_default()
            .push((node.pos, node.biomass));
    }
    groups
}

/// Pick the node closest to the centroid of the group as root.
#[must_use]
fn pick_root_node(nodes: &[(Hex, f32)], layout: &HexLayout) -> Hex {
    assert!(!nodes.is_empty(), "cannot pick root from empty node list");

    #[allow(clippy::cast_precision_loss)]
    let centroid = nodes
        .iter()
        .map(|(pos, _)| layout.hex_to_world_pos(*pos))
        .sum::<Vec2>()
        / nodes.len() as f32;

    nodes
        .iter()
        .min_by(|(a, _), (b, _)| {
            let da = layout.hex_to_world_pos(*a).distance_squared(centroid);
            let db = layout.hex_to_world_pos(*b).distance_squared(centroid);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(pos, _)| *pos)
        .expect("nodes is non-empty")
}

/// BFS from root through edges that connect nodes in `node_set`.
/// Returns (parent, child) directed edge pairs in BFS order.
#[must_use]
fn bfs_edges(root: Hex, node_set: &HashSet<Hex>, edges: &[BranchEdge]) -> Vec<(Hex, Hex)> {
    // Build adjacency list restricted to node_set
    let mut adjacency: HashMap<Hex, Vec<Hex>> = HashMap::new();
    for edge in edges {
        if node_set.contains(&edge.from) && node_set.contains(&edge.to) {
            adjacency.entry(edge.from).or_default().push(edge.to);
            adjacency.entry(edge.to).or_default().push(edge.from);
        }
    }

    let mut visited = HashSet::new();
    visited.insert(root);
    let mut queue = VecDeque::new();
    queue.push_back(root);
    let mut result = Vec::new();

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = adjacency.get(&current) {
            for &neighbor in neighbors {
                if visited.insert(neighbor) {
                    result.push((current, neighbor));
                    queue.push_back(neighbor);
                }
            }
        }
    }

    result
}

/// Generate 0-2 short decorative sub-branches at a node, scaling with biomass.
/// Each branch is returned as (start, end) in world coordinates.
#[must_use]
fn generate_decorative_branches(
    world_pos: Vec2,
    main_dir: Vec2,
    biomass: f32,
    seed: u32,
    hex_outer_radius: f32,
) -> Vec<(Vec2, Vec2)> {
    // Determine branch count: 0 for low biomass, up to 2 for high
    let hash0 = seed.wrapping_mul(2_654_435_761);
    #[allow(clippy::cast_precision_loss)]
    let rand0 = hash0 as f32 / u32::MAX as f32;

    // biomass < 1.0 -> 0 branches, 1.0..3.0 -> 0-1, 3.0+ -> 1-2
    let max_branches = if biomass < 1.0 {
        0
    } else if biomass < 3.0 {
        if rand0 < 0.5 {
            1
        } else {
            0
        }
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            1 + (rand0 * 1.5).min(1.0) as usize
        }
    };

    let mut branches = Vec::with_capacity(max_branches);
    let perp = Vec2::new(-main_dir.y, main_dir.x);

    for i in 0..max_branches {
        #[allow(clippy::cast_possible_truncation)]
        let h = seed
            .wrapping_mul(73_856_093)
            .wrapping_add(i as u32 * 19_349_669);
        #[allow(clippy::cast_precision_loss)]
        let angle_frac = (h as f32 / u32::MAX as f32) * 2.0 - 1.0;

        // Length: 0.5 to 1.5 hex radii
        let h2 = h.wrapping_mul(2_654_435_761);
        #[allow(clippy::cast_precision_loss)]
        let len_frac = h2 as f32 / u32::MAX as f32;
        let length = (0.5 + len_frac) * hex_outer_radius;

        // Direction is a blend of main_dir forward component + perpendicular splay
        let dir = (main_dir * 0.3 + perp * angle_frac).normalize_or_zero();
        let end = world_pos + dir * length;
        branches.push((world_pos, end));
    }

    branches
}

/// Generate 2-3 daughter forks at a leaf tip node, splaying outward.
#[must_use]
fn generate_tip_forks(
    world_pos: Vec2,
    approach_dir: Vec2,
    seed: u32,
    hex_outer_radius: f32,
) -> Vec<(Vec2, Vec2)> {
    let hash0 = seed.wrapping_mul(2_654_435_761);
    #[allow(clippy::cast_precision_loss)]
    let count = if (hash0 as f32 / u32::MAX as f32) < 0.5 {
        2
    } else {
        3
    };

    let mut forks = Vec::with_capacity(count);
    let perp = Vec2::new(-approach_dir.y, approach_dir.x);

    for i in 0..count {
        #[allow(clippy::cast_possible_truncation)]
        let h = seed
            .wrapping_mul(73_856_093)
            .wrapping_add(i as u32 * 19_349_669);
        #[allow(clippy::cast_precision_loss)]
        let splay = (h as f32 / u32::MAX as f32) - 0.5; // [-0.5, 0.5]

        // Spread the daughters: offset angle from center
        #[allow(clippy::cast_precision_loss)]
        let base_spread = (i as f32 / (count as f32 - 0.5)) - 0.5; // roughly [-0.5, 0.5]

        let dir = (approach_dir * 0.7 + perp * (base_spread + splay * 0.3)).normalize_or_zero();
        let length = hex_outer_radius * (0.4 + splay.abs() * 0.4);
        let end = world_pos + dir * length;
        forks.push((world_pos, end));
    }

    forks
}

/// Build all spline meshes for a branch tree from a set of nodes and edges.
///
/// `max_decorative` controls decorative sub-branch count (0 for rivals).
/// `tip_fork` enables tip forking at degree-1 leaf nodes.
#[must_use]
fn build_branch_tree(
    nodes: &[(Hex, f32)],
    edges: &[BranchEdge],
    max_decorative: usize,
    tip_fork: bool,
    layout: &HexLayout,
) -> Vec<Mesh> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let hex_outer_radius = layout.scale.x;

    let node_set: HashSet<Hex> = nodes.iter().map(|(pos, _)| *pos).collect();
    let biomass_map: HashMap<Hex, f32> = nodes.iter().copied().collect();
    let root = pick_root_node(nodes, layout);
    let tree_edges = bfs_edges(root, &node_set, edges);

    // Compute degrees on BFS tree edges (not original graph) for leaf detection
    let mut degrees: HashMap<Hex, usize> = HashMap::new();
    for (parent, child) in &tree_edges {
        *degrees.entry(*parent).or_default() += 1;
        *degrees.entry(*child).or_default() += 1;
    }

    let mut result: Vec<Mesh> = Vec::new();

    for (idx, (parent, child)) in tree_edges.iter().enumerate() {
        let from_world = layout.hex_to_world_pos(*parent);
        let to_world = layout.hex_to_world_pos(*child);

        // Generate STRANDS_PER_EDGE spline strands per edge
        for strand in 0..STRANDS_PER_EDGE {
            #[allow(clippy::cast_possible_truncation)]
            let seed = (idx as u32)
                .wrapping_mul(2_654_435_761)
                .wrapping_add(strand as u32 * 73_856_093);
            let (mesh, _points) =
                build_spline_mesh_with_wobble(from_world, to_world, STRAND_HALF_WIDTH, seed);
            result.push(mesh);
        }

        // Decorative branches at child node
        if max_decorative > 0 {
            let dir = (to_world - from_world).normalize_or_zero();
            let child_biomass = biomass_map.get(child).copied().unwrap_or(1.0);
            #[allow(clippy::cast_possible_truncation)]
            let deco_seed = (idx as u32).wrapping_mul(19_349_669);
            let decos = generate_decorative_branches(
                to_world,
                dir,
                child_biomass,
                deco_seed,
                hex_outer_radius,
            );
            for (deco_start, deco_end) in decos.into_iter().take(max_decorative) {
                let (mesh, _) = build_spline_mesh_with_wobble(
                    deco_start,
                    deco_end,
                    STRAND_HALF_WIDTH * 0.7,
                    deco_seed.wrapping_add(42),
                );
                result.push(mesh);
            }
        }

        // Tip forks at degree-1 leaf child nodes
        if tip_fork {
            let child_degree = degrees.get(child).copied().unwrap_or(0);
            if child_degree == 1 {
                let dir = (to_world - from_world).normalize_or_zero();
                #[allow(clippy::cast_possible_truncation)]
                let fork_seed = (idx as u32).wrapping_mul(91_939_117);
                let forks = generate_tip_forks(to_world, dir, fork_seed, hex_outer_radius);
                for (fork_start, fork_end) in forks {
                    let (mesh, _) = build_spline_mesh_with_wobble(
                        fork_start,
                        fork_end,
                        STRAND_HALF_WIDTH * 0.5,
                        fork_seed.wrapping_add(7),
                    );
                    result.push(mesh);
                }
            }
        }
    }

    result
}

#[derive(SystemParam)]
pub struct NetworkAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<NetworkMaterial>>,
}

pub fn network_render_system(
    mut commands: Commands,
    graph: Res<BranchGraph>,
    rival_graph: Res<RivalBranchGraph>,
    existing: Query<Entity, With<BranchTreeMesh>>,
    mut assets: NetworkAssets,
    time: Res<Time>,
    layout: Res<HexLayout>,
) {
    if !graph.is_changed() && !rival_graph.is_changed() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    let elapsed = time.elapsed_secs();

    // --- Player network: group by region, build tree per region ---
    let player_groups = group_player_nodes_by_region(&graph);
    for (region_id, region_nodes) in &player_groups {
        // Determine specialization from any node in this region
        let spec = graph
            .nodes
            .values()
            .find(|n| n.region_id == *region_id)
            .and_then(|n| n.specialization);

        let core = region_color_linear(spec);
        let body = body_color_from_core(core);
        let avg_biomass = if region_nodes.is_empty() {
            1.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                region_nodes.iter().map(|(_, b)| b).sum::<f32>() / region_nodes.len() as f32
            }
        };

        let tree_meshes = build_branch_tree(region_nodes, &graph.edges, 2, true, &layout);

        for mesh in tree_meshes {
            commands.spawn((
                BranchTreeMesh,
                Mesh2d(assets.meshes.add(mesh)),
                MeshMaterial2d(assets.materials.add(NetworkMaterial {
                    uniforms: NetworkUniforms {
                        core_color: core,
                        body_color: body,
                        biomass: avg_biomass,
                        time: elapsed,
                        _padding: Vec2::ZERO,
                    },
                })),
                Transform::from_translation(Vec3::ZERO.with_z(1.0)),
            ));
        }
    }

    // --- Rival network: group by rival_id, build tree without decoratives ---
    let rival_core = LinearRgba::new(0.7, 0.1, 0.1, 1.0);
    let rival_body = LinearRgba::new(0.3, 0.05, 0.05, 0.7);

    let rival_groups = group_rival_nodes_by_id(&rival_graph);
    for rival_nodes in rival_groups.values() {
        let avg_biomass = if rival_nodes.is_empty() {
            1.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                rival_nodes.iter().map(|(_, b)| b).sum::<f32>() / rival_nodes.len() as f32
            }
        };

        let tree_meshes = build_branch_tree(rival_nodes, &rival_graph.edges, 0, false, &layout);

        for mesh in tree_meshes {
            commands.spawn((
                BranchTreeMesh,
                Mesh2d(assets.meshes.add(mesh)),
                MeshMaterial2d(assets.materials.add(NetworkMaterial {
                    uniforms: NetworkUniforms {
                        core_color: rival_core,
                        body_color: rival_body,
                        biomass: avg_biomass,
                        time: elapsed,
                        _padding: Vec2::ZERO,
                    },
                })),
                Transform::from_translation(Vec3::ZERO.with_z(1.0)),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fungai_core::{create_hex_layout, SpecializationType};

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

        let left = Vec2::new(verts[0][0], verts[0][1]);
        let right = Vec2::new(verts[1][0], verts[1][1]);
        let midpoint = (left + right) * 0.5;

        assert!(
            (midpoint - from).length() < 0.01,
            "midpoint {midpoint} should be near from {from}"
        );

        let separation = (left - right).length();
        assert!(
            (separation - half_width * 2.0).abs() < 0.01,
            "separation {separation} should be near {half_width_2}",
            half_width_2 = half_width * 2.0
        );
    }

    #[test]
    fn region_color_maps_specializations() {
        let explorer = region_color_linear(Some(SpecializationType::Explorer));
        assert_eq!(explorer, LinearRgba::new(1.0, 0.9, 0.3, 1.0));

        let parasite = region_color_linear(Some(SpecializationType::Parasite));
        assert_eq!(parasite, LinearRgba::new(0.8, 0.2, 0.2, 1.0));

        let none_color = region_color_linear(None);
        assert_eq!(none_color, LinearRgba::new(0.9, 0.85, 0.7, 1.0));

        let hunter = region_color_linear(Some(SpecializationType::Hunter));
        assert_eq!(hunter, LinearRgba::new(0.6, 0.4, 0.1, 1.0));
    }

    // --- Step 1: UV attribute tests ---

    #[test]
    fn spline_mesh_has_uv_attribute() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _) = build_spline_mesh(from, to, 4.0);

        let uvs = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .expect("mesh should have UV_0 attribute");

        let uv_count = match uvs {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => v.len(),
            _ => panic!("unexpected UV format"),
        };
        assert_eq!(uv_count, SPLINE_SAMPLES * 2);
    }

    #[test]
    fn spline_mesh_uv_range_is_symmetric() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _) = build_spline_mesh(from, to, 4.0);

        let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap() {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => v.clone(),
            _ => panic!("unexpected UV format"),
        };

        // First left vertex: u = -1, v = 0
        assert!((uvs[0][0] - (-1.0)).abs() < 0.001, "left u should be -1");
        assert!((uvs[0][1]).abs() < 0.001, "first v should be 0");
        // First right vertex: u = 1, v = 0
        assert!((uvs[1][0] - 1.0).abs() < 0.001, "right u should be 1");
        // Last right vertex: v should be 1
        assert!(
            (uvs[SPLINE_SAMPLES * 2 - 1][1] - 1.0).abs() < 0.001,
            "last v should be 1"
        );
    }

    // --- Step 2: Wobble tests ---

    #[test]
    fn spline_mesh_with_wobble_differs_from_straight() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (_, straight_points) = build_spline_mesh(from, to, 4.0);
        let (_, wobble_points) = build_spline_mesh_with_wobble(from, to, 4.0, 42);

        let mut any_different = false;
        for i in 1..(SPLINE_SAMPLES - 1) {
            if (straight_points[i] - wobble_points[i]).length() > 0.01 {
                any_different = true;
                break;
            }
        }
        assert!(
            any_different,
            "wobble should displace at least one interior point"
        );
    }

    #[test]
    fn spline_mesh_wobble_preserves_endpoints() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (_, straight_points) = build_spline_mesh(from, to, 4.0);
        let (_, wobble_points) = build_spline_mesh_with_wobble(from, to, 4.0, 99);

        assert!(
            (straight_points[0] - wobble_points[0]).length() < 0.001,
            "start endpoint must not be displaced"
        );
        assert!(
            (straight_points[SPLINE_SAMPLES - 1] - wobble_points[SPLINE_SAMPLES - 1]).length()
                < 0.001,
            "end endpoint must not be displaced"
        );
    }

    #[test]
    fn spline_mesh_wobble_has_uv_attribute() {
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let (mesh, _) = build_spline_mesh_with_wobble(from, to, 4.0, 7);

        let uvs = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .expect("wobble mesh should have UV_0 attribute");

        let uv_count = match uvs {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => v.len(),
            _ => panic!("unexpected UV format"),
        };
        assert_eq!(uv_count, SPLINE_SAMPLES * 2);
    }

    // --- Step 3: NetworkMaterial tests ---

    #[test]
    fn network_material_stores_core_color_and_biomass() {
        let mat = NetworkMaterial {
            uniforms: NetworkUniforms {
                core_color: LinearRgba::new(1.0, 0.9, 0.3, 1.0),
                body_color: LinearRgba::new(0.5, 0.45, 0.15, 0.6),
                biomass: 5.0,
                time: 0.0,
                _padding: Vec2::ZERO,
            },
        };
        assert_eq!(mat.uniforms.biomass, 5.0);
    }

    #[test]
    fn body_color_from_core_is_muted() {
        let core = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
        let body = body_color_from_core(core);
        assert!(body.red < core.red, "body red should be less than core red");
        assert_eq!(body.alpha, 0.7);
    }

    // --- Step 1.1: BranchTreeMesh component ---

    #[test]
    fn branch_tree_mesh_is_component() {
        let mut world = World::new();
        let entity = world.spawn(BranchTreeMesh).id();
        assert!(world.get::<BranchTreeMesh>(entity).is_some());
    }

    // --- Step 1.2: compute_node_degrees ---

    #[test]
    fn compute_node_degrees_counts_edges() {
        let mut graph = BranchGraph::default();
        let a = Hex::new(0, 0);
        let b = Hex::new(1, 0);
        let c = Hex::new(2, 0);

        graph.edges.push(BranchEdge {
            from: a,
            to: b,
            thickness: 1.0,
        });
        graph.edges.push(BranchEdge {
            from: b,
            to: c,
            thickness: 1.0,
        });

        let degrees = compute_node_degrees(&graph);
        assert_eq!(degrees[&a], 1);
        assert_eq!(degrees[&b], 2);
        assert_eq!(degrees[&c], 1);
    }

    // --- Step 1.3: Grouping functions ---

    #[test]
    fn group_player_nodes_by_region_groups_correctly() {
        use crate::data_layer::BranchNode;

        let mut graph = BranchGraph::default();
        let r1 = RegionId(1);
        let r2 = RegionId(2);

        graph.nodes.insert(
            Hex::new(0, 0),
            BranchNode {
                pos: Hex::new(0, 0),
                biomass: 1.0,
                specialization: None,
                region_id: r1,
            },
        );
        graph.nodes.insert(
            Hex::new(1, 0),
            BranchNode {
                pos: Hex::new(1, 0),
                biomass: 2.0,
                specialization: None,
                region_id: r1,
            },
        );
        graph.nodes.insert(
            Hex::new(5, 5),
            BranchNode {
                pos: Hex::new(5, 5),
                biomass: 3.0,
                specialization: None,
                region_id: r2,
            },
        );

        let groups = group_player_nodes_by_region(&graph);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[&r1].len(), 2);
        assert_eq!(groups[&r2].len(), 1);
    }

    #[test]
    fn group_rival_nodes_by_id_groups_correctly() {
        use crate::data_layer::RivalBranchNode;

        let mut graph = RivalBranchGraph::default();
        let r1 = RivalId(0);
        let r2 = RivalId(1);

        graph.nodes.insert(
            Hex::new(0, 0),
            RivalBranchNode {
                pos: Hex::new(0, 0),
                biomass: 1.0,
                rival_id: r1,
            },
        );
        graph.nodes.insert(
            Hex::new(1, 0),
            RivalBranchNode {
                pos: Hex::new(1, 0),
                biomass: 2.0,
                rival_id: r2,
            },
        );

        let groups = group_rival_nodes_by_id(&graph);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[&r1].len(), 1);
        assert_eq!(groups[&r2].len(), 1);
    }

    // --- Step 1.4: pick_root_node and bfs_edges ---

    #[test]
    fn pick_root_node_selects_centroid_closest() {
        let layout = create_hex_layout();
        let nodes = vec![
            (Hex::new(0, 0), 1.0),
            (Hex::new(2, 0), 1.0),
            (Hex::new(4, 0), 1.0),
        ];
        // Centroid in world coords is closest to Hex(2,0)
        let root = pick_root_node(&nodes, &layout);
        assert_eq!(root, Hex::new(2, 0));
    }

    #[test]
    fn bfs_edges_traverses_connected_graph() {
        let nodes: HashSet<Hex> = [Hex::new(0, 0), Hex::new(1, 0), Hex::new(2, 0)]
            .into_iter()
            .collect();
        let edges = vec![
            BranchEdge {
                from: Hex::new(0, 0),
                to: Hex::new(1, 0),
                thickness: 1.0,
            },
            BranchEdge {
                from: Hex::new(1, 0),
                to: Hex::new(2, 0),
                thickness: 1.0,
            },
        ];

        let result = bfs_edges(Hex::new(0, 0), &nodes, &edges);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (Hex::new(0, 0), Hex::new(1, 0)));
        assert_eq!(result[1], (Hex::new(1, 0), Hex::new(2, 0)));
    }

    #[test]
    fn bfs_edges_skips_nodes_outside_set() {
        let nodes: HashSet<Hex> = [Hex::new(0, 0), Hex::new(1, 0)].into_iter().collect();
        let edges = vec![
            BranchEdge {
                from: Hex::new(0, 0),
                to: Hex::new(1, 0),
                thickness: 1.0,
            },
            BranchEdge {
                from: Hex::new(1, 0),
                to: Hex::new(2, 0), // not in set
                thickness: 1.0,
            },
        ];

        let result = bfs_edges(Hex::new(0, 0), &nodes, &edges);
        assert_eq!(result.len(), 1);
    }

    // --- Step 1.5: Decorative branches ---

    #[test]
    fn decorative_branches_scale_with_biomass() {
        let hex_outer_radius = 28.0;
        let low = generate_decorative_branches(
            Vec2::new(48.0, 48.0),
            Vec2::new(0.0, 1.0),
            0.5,
            42,
            hex_outer_radius,
        );
        let high = generate_decorative_branches(
            Vec2::new(48.0, 48.0),
            Vec2::new(0.0, 1.0),
            5.0,
            42,
            hex_outer_radius,
        );
        assert!(high.len() >= low.len());
    }

    #[test]
    fn decorative_branches_are_short() {
        let hex_outer_radius = 28.0;
        let branches = generate_decorative_branches(
            Vec2::new(48.0, 48.0),
            Vec2::new(0.0, 1.0),
            3.0,
            7,
            hex_outer_radius,
        );
        for (start, end) in &branches {
            let length = (*end - *start).length();
            assert!(
                length <= 1.5 * hex_outer_radius + 1.0,
                "decorative branch too long: {length}"
            );
        }
    }

    // --- Step 1.6: Tip forking ---

    #[test]
    fn tip_forks_produce_2_to_3_daughters() {
        let hex_outer_radius = 28.0;
        let forks = generate_tip_forks(
            Vec2::new(96.0, 96.0),
            Vec2::new(1.0, 0.0),
            42,
            hex_outer_radius,
        );
        assert!(
            forks.len() >= 2 && forks.len() <= 3,
            "expected 2-3 forks, got {}",
            forks.len()
        );
    }

    #[test]
    fn tip_forks_splay_outward() {
        let parent_dir = Vec2::new(1.0, 0.0);
        let hex_outer_radius = 28.0;
        let forks = generate_tip_forks(Vec2::new(0.0, 0.0), parent_dir, 42, hex_outer_radius);
        for (start, end) in &forks {
            let fork_dir = (*end - *start).normalize_or_zero();
            let dot = fork_dir.dot(parent_dir);
            assert!(
                dot > -0.3,
                "fork should splay outward, not backward: dot={dot}"
            );
        }
    }

    // --- Step 1.7: build_branch_tree ---

    #[test]
    fn build_branch_tree_produces_meshes_for_edges() {
        let layout = create_hex_layout();
        let nodes = vec![
            (Hex::new(0, 0), 1.0),
            (Hex::new(1, 0), 2.0),
            (Hex::new(2, 0), 1.0),
        ];
        let edges = vec![
            BranchEdge {
                from: Hex::new(0, 0),
                to: Hex::new(1, 0),
                thickness: 1.0,
            },
            BranchEdge {
                from: Hex::new(1, 0),
                to: Hex::new(2, 0),
                thickness: 1.0,
            },
        ];

        let result = build_branch_tree(&nodes, &edges, 2, true, &layout);
        // At least 2 edges * STRANDS_PER_EDGE strands
        assert!(
            result.len() >= 2 * STRANDS_PER_EDGE,
            "expected at least {} meshes, got {}",
            2 * STRANDS_PER_EDGE,
            result.len()
        );
    }

    #[test]
    fn build_branch_tree_no_decoratives_for_rivals() {
        let layout = create_hex_layout();
        let nodes = vec![
            (Hex::new(0, 0), 3.0),
            (Hex::new(1, 0), 3.0),
            (Hex::new(2, 0), 3.0),
        ];
        let edges = vec![
            BranchEdge {
                from: Hex::new(0, 0),
                to: Hex::new(1, 0),
                thickness: 1.0,
            },
            BranchEdge {
                from: Hex::new(1, 0),
                to: Hex::new(2, 0),
                thickness: 1.0,
            },
        ];

        let with_deco = build_branch_tree(&nodes, &edges, 2, true, &layout);
        let without_deco = build_branch_tree(&nodes, &edges, 0, false, &layout);
        assert!(
            with_deco.len() >= without_deco.len(),
            "decoratives should add meshes: with={} without={}",
            with_deco.len(),
            without_deco.len()
        );
    }
}
