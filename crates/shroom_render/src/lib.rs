use bevy::prelude::*;

mod data_layer;
mod entity_render;
mod network_render;
mod terrain_render;

pub use data_layer::{BranchGraph, RegionHulls, TipPositions};
pub use network_render::catmull_rom;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BranchGraph>()
            .init_resource::<TipPositions>()
            .init_resource::<RegionHulls>()
            .add_systems(
                Update,
                (
                    data_layer::extract_branch_graph,
                    data_layer::extract_tip_positions,
                    data_layer::extract_region_hulls,
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    terrain_render::terrain_render_system,
                    network_render::network_render_system,
                    entity_render::tip_render_system,
                    entity_render::organism_render_system,
                ),
            );
    }
}
