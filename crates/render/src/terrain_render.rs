use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::Material2d,
};
use fungai_core::*;
use hexx::PlaneMeshBuilder;

/// Packed uniform struct -- matches the WGSL `TerrainUniforms` struct exactly.
#[derive(ShaderType, Debug, Clone)]
pub struct TerrainUniforms {
    pub base_color: LinearRgba, // vec4<f32> -- 16 bytes
    pub terrain_type: u32,      // u32 -- 4 bytes
    pub grid_x: u32,            // u32 -- 4 bytes (axial q coordinate, used for noise seed)
    pub grid_y: u32,            // u32 -- 4 bytes (axial r coordinate, used for noise seed)
    pub discovered: f32,        // f32 -- 4 bytes
    pub time: f32,              // f32 -- 4 bytes
    pub nutrient_level: f32,    // f32 -- 4 bytes
    pub _padding: f32,          // pad to 16-byte boundary -- 4 bytes
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub uniforms: TerrainUniforms,
}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

#[derive(Component)]
pub struct TerrainMeshTile {
    pub grid_pos: Hex,
}

pub fn terrain_base_color(terrain: TerrainType) -> LinearRgba {
    match terrain {
        TerrainType::Soil => LinearRgba::new(0.18, 0.12, 0.07, 1.0),
        TerrainType::Rock => LinearRgba::new(0.20, 0.20, 0.22, 1.0),
        TerrainType::Water => LinearRgba::new(0.06, 0.12, 0.30, 1.0),
        TerrainType::Root => LinearRgba::new(0.10, 0.18, 0.08, 1.0),
        TerrainType::Ruin => LinearRgba::new(0.22, 0.20, 0.14, 1.0),
        TerrainType::Toxic => LinearRgba::new(0.18, 0.28, 0.05, 1.0),
        TerrainType::Surface => LinearRgba::new(0.10, 0.22, 0.10, 1.0),
    }
}

pub fn terrain_type_index(terrain: TerrainType) -> u32 {
    match terrain {
        TerrainType::Soil => 0,
        TerrainType::Rock => 1,
        TerrainType::Water => 2,
        TerrainType::Root => 3,
        TerrainType::Ruin => 4,
        TerrainType::Toxic => 5,
        TerrainType::Surface => 6,
    }
}

/// Build a 2D hex mesh from the layout. Uses `PlaneMeshBuilder` with Z-facing
/// orientation so the hex lies flat on the XY plane for Bevy 2D.
fn build_hex_mesh(layout: &HexLayout) -> Mesh {
    let mesh_info = PlaneMeshBuilder::new(layout).facing(Vec3::Z).build();

    Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh_info.vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, mesh_info.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh_info.uvs)
    .with_inserted_indices(bevy::mesh::Indices::U16(mesh_info.indices))
}

#[derive(SystemParam)]
pub struct TerrainAssets<'w> {
    sprite_map: ResMut<'w, TerrainSpriteMap>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<TerrainMaterial>>,
}

pub fn terrain_render_system(
    mut commands: Commands,
    tiles: Query<(&GridPos, &Tile), Changed<Tile>>,
    mut assets: TerrainAssets,
    time: Res<Time>,
    discovery: Res<crate::data_layer::DiscoveryMap>,
    layout: Res<HexLayout>,
) {
    for (gpos, tile) in tiles.iter() {
        if let Some(old_entity) = assets.sprite_map.sprites.remove(&gpos.0) {
            commands.entity(old_entity).despawn();
        }

        let base_color = terrain_base_color(tile.terrain);
        let t_index = terrain_type_index(tile.terrain);

        let base_pos = layout.hex_to_world_pos(gpos.0);
        let world_pos = Vec3::new(base_pos.x, base_pos.y, 0.0);

        let material = assets.materials.add(TerrainMaterial {
            uniforms: TerrainUniforms {
                base_color,
                terrain_type: t_index,
                grid_x: gpos.0.x as u32,
                grid_y: gpos.0.y as u32,
                discovered: discovery.discovered.get(&gpos.0).copied().unwrap_or(0.0),
                time: time.elapsed_secs(),
                nutrient_level: tile.nutrient_level,
                _padding: 0.0,
            },
        });

        let entity = commands
            .spawn((
                TerrainMeshTile { grid_pos: gpos.0 },
                Mesh2d(assets.meshes.add(build_hex_mesh(&layout))),
                MeshMaterial2d(material),
                Transform::from_translation(world_pos).with_scale(Vec3::splat(1.01)),
            ))
            .id();
        assets.sprite_map.sprites.insert(gpos.0, entity);
    }
}

/// Updates discovery and time uniforms on existing terrain materials each frame.
pub fn terrain_discovery_update_system(
    tiles: Query<(&TerrainMeshTile, &MeshMaterial2d<TerrainMaterial>)>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    discovery: Res<crate::data_layer::DiscoveryMap>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    for (tile, mat_handle) in tiles.iter() {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.uniforms.discovered = discovery
                .discovered
                .get(&tile.grid_pos)
                .copied()
                .unwrap_or(0.0);
            mat.uniforms.time = elapsed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::MinimalPlugins;
    use bevy::asset::AssetPlugin;
    use bevy::sprite_render::Material2dPlugin;

    #[test]
    fn terrain_material_stores_uniforms() {
        let uniforms = TerrainUniforms {
            base_color: LinearRgba::new(0.45, 0.32, 0.18, 1.0),
            terrain_type: 0,
            grid_x: 5,
            grid_y: 10,
            discovered: 1.0,
            time: 0.0,
            nutrient_level: 0.5,
            _padding: 0.0,
        };
        let mat = TerrainMaterial { uniforms };
        assert_eq!(mat.uniforms.terrain_type, 0);
        assert_eq!(mat.uniforms.grid_x, 5);
        assert_eq!(mat.uniforms.grid_y, 10);
    }

    #[test]
    fn terrain_base_color_returns_dark_palette() {
        let soil = terrain_base_color(TerrainType::Soil);
        assert!(soil.red < 0.25);
        assert!(soil.green < 0.20);
        assert!(soil.blue < 0.15);

        let water = terrain_base_color(TerrainType::Water);
        assert!(water.blue > water.red);
        assert!(water.blue > water.green);
    }

    #[test]
    fn terrain_render_spawns_mesh2d_entities() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(bevy::mesh::MeshPlugin);
        app.add_plugins(Material2dPlugin::<TerrainMaterial>::default());
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<TerrainSpriteMap>();
        app.init_resource::<crate::data_layer::DiscoveryMap>();
        app.insert_resource(create_hex_layout());

        let pos = Hex::new(3, 4);
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    terrain: TerrainType::Rock,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);

        app.add_systems(PostUpdate, terrain_render_system);
        app.update();

        let sprite_map = app.world().resource::<TerrainSpriteMap>();
        assert!(sprite_map.sprites.contains_key(&pos));
    }
}
