use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RegionId(pub u32);

#[derive(Clone, Debug, Reflect)]
pub struct RegionState {
    pub region_id: RegionId,
    pub sugars: f32,
    pub melanin: f32,
    pub total_biomass: f32,
    pub tile_count: u32,
}

impl RegionState {
    pub fn new(id: RegionId) -> Self {
        Self {
            region_id: id,
            sugars: 10.0,
            melanin: 0.0,
            total_biomass: 0.0,
            tile_count: 0,
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct RegionStates {
    pub regions: HashMap<RegionId, RegionState>,
    next_id: u32,
}

impl RegionStates {
    pub fn create_region(&mut self) -> RegionId {
        let id = RegionId(self.next_id);
        self.next_id += 1;
        self.regions.insert(id, RegionState::new(id));
        id
    }

    pub fn get(&self, id: RegionId) -> Option<&RegionState> {
        self.regions.get(&id)
    }

    pub fn get_mut(&mut self, id: RegionId) -> Option<&mut RegionState> {
        self.regions.get_mut(&id)
    }

    pub fn remove(&mut self, id: RegionId) -> Option<RegionState> {
        self.regions.remove(&id)
    }
}
