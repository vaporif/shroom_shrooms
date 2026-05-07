use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy_ecs_tilemap::prelude::TilemapPlugin;
use kingdom_core::SimulationSystems;
use kingdom_world::terrain_generation;

mod assets;
mod atmosphere;
mod data_layer;
mod entity_render;
mod network_render;
mod terrain_render;

pub use data_layer::{BranchGraph, DiscoveryMap, RegionHulls};
pub use network_render::catmull_rom;
pub use terrain_render::{terrain_base_color, terrain_type_index};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .add_plugins(Material2dPlugin::<atmosphere::VignetteMaterial>::default())
            .add_plugins(Material2dPlugin::<network_render::NetworkMaterial>::default())
            .init_resource::<assets::EntitySprites>()
            .init_resource::<terrain_render::PendingAtlasCheck>()
            .init_resource::<BranchGraph>()
            .init_resource::<RegionHulls>()
            .init_resource::<data_layer::DiscoveryMap>()
            .init_resource::<data_layer::SelectedRegionTiles>()
            .init_resource::<data_layer::SelectedRegionExtractionRuns>()
            .add_systems(
                Update,
                (
                    data_layer::extract_branch_graph,
                    data_layer::extract_region_hulls,
                    data_layer::extract_discovery_map.after(data_layer::extract_branch_graph),
                )
                    .in_set(SimulationSystems),
            )
            .add_systems(
                Update,
                (
                    data_layer::extract_selected_region_tiles,
                    terrain_render::assert_atlas_addresses_all_terrains,
                ),
            )
            .add_systems(
                Startup,
                (
                    assets::load_entity_sprites,
                    atmosphere::spawn_vignette,
                    atmosphere::spawn_particle_pool,
                    terrain_render::spawn_terrain_tilemap.after(terrain_generation),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    terrain_render::terrain_tile_update_system,
                    network_render::network_render_system,
                    (
                        entity_render::despawn_orphaned_organism_sprites,
                        entity_render::spawn_organism_sprites,
                    )
                        .chain(),
                    entity_render::bias_glow_render_system,
                    entity_render::region_highlight_render_system,
                    atmosphere::update_vignette,
                    atmosphere::update_particles,
                ),
            );
    }
}
