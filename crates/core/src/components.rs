use bevy::prelude::*;

use crate::grid::Hex;
use crate::region::RegionId;
use crate::tile::FragmentId;

#[derive(Component, Clone, Debug, Reflect)]
pub struct FaunaAgent {
    pub health: f32,
    pub damage_per_tick: f32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct BacteriaColonyAgent {
    pub spread_timer: u32,
    pub spread_interval: u32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct PlantRootAgent {
    pub plant_id: u32,
    pub health: f32,
    pub trade_active: bool,
    pub nutrient_intake: f32,
    pub sugar_output: f32,
    pub neglect_timer: u32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct NeutralFungusAgent {
    pub fungus_id: u32,
    pub merge_progress: f32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct FragmentAgent {
    pub fragment_id: FragmentId,
    pub fused: bool,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct FruitingBody {
    pub region_id: RegionId,
    pub fragment_id: FragmentId,
    pub progress: f32,
    #[reflect(ignore)]
    pub column_top: Hex,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct MushroomEntity {
    pub fragment_id: FragmentId,
    #[reflect(ignore)]
    pub pos: Hex,
    pub vision_radius: f32,
}

#[derive(Component, Debug)]
pub struct OrganismSpriteLink(pub Entity);

#[derive(Resource, Default)]
pub struct SelectedRegion {
    pub region_id: Option<RegionId>,
    pub selected_pos: Option<Hex>,
}
