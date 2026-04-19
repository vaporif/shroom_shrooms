use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

mod assets;
mod atmosphere;
mod data_layer;
mod entity_render;
mod network_render;
mod terrain_render;

pub use data_layer::{BranchGraph, DiscoveryMap, RegionHulls, RivalBranchGraph, TipPositions};
pub use network_render::catmull_rom;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<terrain_render::TerrainMaterial>::default())
            .add_plugins(Material2dPlugin::<atmosphere::VignetteMaterial>::default())
            .add_plugins(Material2dPlugin::<network_render::NetworkMaterial>::default())
            .init_resource::<assets::EntitySprites>()
            .init_resource::<BranchGraph>()
            .init_resource::<TipPositions>()
            .init_resource::<RegionHulls>()
            .init_resource::<data_layer::DiscoveryMap>()
            .init_resource::<data_layer::RivalBranchGraph>()
            .add_systems(
                Update,
                (
                    data_layer::extract_branch_graph,
                    data_layer::extract_tip_positions,
                    data_layer::extract_region_hulls,
                    data_layer::extract_discovery_map.after(data_layer::extract_branch_graph),
                    data_layer::extract_rival_branch_graph,
                ),
            )
            .add_systems(
                Startup,
                (
                    assets::load_entity_sprites,
                    atmosphere::spawn_vignette,
                    atmosphere::spawn_particle_pool,
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    terrain_render::terrain_render_system,
                    terrain_render::terrain_discovery_update_system
                        .after(terrain_render::terrain_render_system),
                    network_render::network_render_system,
                    entity_render::tip_render_system,
                    entity_render::organism_render_system,
                    atmosphere::update_particles,
                ),
            );
    }
}
