use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::Material2d,
};
use shroom_core::*;

pub const TILE_SIZE: f32 = 48.0;

/// Packed uniform struct — matches the WGSL `TerrainUniforms` struct exactly.
#[derive(ShaderType, Debug, Clone)]
pub struct TerrainUniforms {
    pub base_color: LinearRgba, // vec4<f32> — 16 bytes
    pub terrain_type: u32,      // u32 — 4 bytes
    pub grid_x: u32,            // u32 — 4 bytes
    pub grid_y: u32,            // u32 — 4 bytes
    pub discovered: f32,        // f32 — 4 bytes
    pub time: f32,              // f32 — 4 bytes
    pub nutrient_level: f32,    // f32 — 4 bytes
    pub _padding: f32,          // pad to 16-byte boundary — 4 bytes
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
    pub grid_pos: IVec2,
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

pub fn terrain_render_system(
    mut commands: Commands,
    tiles: Query<(&GridPos, &Tile), Changed<Tile>>,
    mut sprite_map: ResMut<TerrainSpriteMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    time: Res<Time>,
    discovery: Res<crate::data_layer::DiscoveryMap>,
) {
    for (gpos, tile) in tiles.iter() {
        if let Some(old_entity) = sprite_map.sprites.remove(&gpos.0) {
            commands.entity(old_entity).despawn();
        }

        let base_color = terrain_base_color(tile.terrain);
        let t_index = terrain_type_index(tile.terrain);

        // Deterministic jitter seeded by grid position
        let seed = (gpos.0.x.wrapping_mul(73_856_093)) ^ (gpos.0.y.wrapping_mul(19_349_663));
        let jitter_x = ((seed & 0xFF) as f32 / 255.0 - 0.5) * 3.0;
        let jitter_y = (((seed >> 8) & 0xFF) as f32 / 255.0 - 0.5) * 3.0;
        let rotation = ((seed >> 16) & 0xFF) as f32 / 255.0 * 0.05 - 0.025;

        let world_pos = Vec3::new(
            gpos.0.x as f32 * TILE_SIZE + jitter_x,
            gpos.0.y as f32 * TILE_SIZE + jitter_y,
            0.0,
        );

        let material = materials.add(TerrainMaterial {
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
                Mesh2d(meshes.add(Rectangle::new(TILE_SIZE, TILE_SIZE))),
                MeshMaterial2d(material),
                Transform::from_translation(world_pos)
                    .with_rotation(Quat::from_rotation_z(rotation)),
            ))
            .id();
        sprite_map.sprites.insert(gpos.0, entity);
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
    use bevy::asset::AssetPlugin;
    use bevy::sprite_render::Material2dPlugin;
    use bevy::MinimalPlugins;

    #[test]
    fn tile_size_is_48() {
        assert_eq!(TILE_SIZE, 48.0);
    }

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

        let pos = IVec2::new(3, 4);
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
