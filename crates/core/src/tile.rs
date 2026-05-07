use bevy::prelude::*;

use crate::region::RegionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
pub enum TerrainType {
    #[default]
    Soil,
    Rock,
    Water,
    Root,
    Ruin,
    Toxic,
    Surface,
}

impl TerrainType {
    pub fn is_passable(&self) -> bool {
        matches!(self, Self::Soil | Self::Root | Self::Ruin | Self::Surface)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct FragmentId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum TileContents {
    OrganicMatter,
    Mineral,
    Artifact,
    Fragment(FragmentId),
    UniqueDecomposable(u32),
    NeutralFungus(u32),
    PlantRoot(u32),
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Tile {
    pub terrain: TerrainType,
    pub region_id: Option<RegionId>,
    pub biomass: f32,
    pub moisture: f32,
    pub radiation: f32,
    pub soil_richness: f32,
    pub nutrient_gradient: Vec2,
    pub priority_bias: Vec2,
    pub discovered: bool,
    pub contents: Option<TileContents>,
}

impl Tile {
    /// True when the tile is claimed by a region with biomass at or above the claim threshold.
    pub fn is_owned(&self) -> bool {
        self.region_id.is_some() && self.biomass >= crate::CLAIM_THRESHOLD
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain: TerrainType::Soil,
            region_id: None,
            biomass: 0.0,
            moisture: 0.5,
            radiation: 0.0,
            soil_richness: 0.5,
            nutrient_gradient: Vec2::ZERO,
            priority_bias: Vec2::ZERO,
            discovered: false,
            contents: None,
        }
    }
}
