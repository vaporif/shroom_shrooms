use bevy::ecs::message::Message;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridWorld>()
            .init_resource::<RegionStates>()
            .init_resource::<GameState>()
            .add_message::<TurnAdvanced>()
            .add_message::<TileDiscovered>()
            .add_message::<StudyComplete>()
            .add_message::<DecompositionComplete>()
            .add_message::<FragmentFused>()
            .add_message::<SlotMachineTriggered>();
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct GridPos(pub IVec2);

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct GridWorld {
    pub tiles: HashMap<IVec2, Entity>,
    pub width: i32,
    pub height: i32,
}

impl GridWorld {
    pub fn neighbors(&self, pos: IVec2) -> impl Iterator<Item = (IVec2, Entity)> + '_ {
        const DIRS: [IVec2; 4] = [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ];
        DIRS.iter().filter_map(move |&d| {
            let neighbor = pos + d;
            self.tiles.get(&neighbor).map(|&e| (neighbor, e))
        })
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
pub enum Occupant {
    #[default]
    Empty,
    Player(RegionId),
    Rival(RivalId),
}

impl Occupant {
    pub fn is_player(&self) -> bool {
        matches!(self, Self::Player(_))
    }

    pub fn is_rival(&self) -> bool {
        matches!(self, Self::Rival(_))
    }

    pub fn region_id(&self) -> Option<RegionId> {
        match self {
            Self::Player(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RegionId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RivalId(pub u32);

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
    pub occupant: Occupant,
    pub nutrient_level: f32,
    pub moisture: f32,
    pub discovered: bool,
    pub contents: Option<TileContents>,
    pub biomass: f32,
    pub nutrient_gradient: Vec2,
    pub priority_bias: Vec2,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain: TerrainType::Soil,
            occupant: Occupant::Empty,
            nutrient_level: 0.5,
            moisture: 0.5,
            discovered: false,
            contents: None,
            biomass: 0.0,
            nutrient_gradient: Vec2::ZERO,
            priority_bias: Vec2::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum SpecializationType {
    Decomposer,
    Parasite,
    Symbiont,
    Infiltrator,
    Hunter,
    Transporter,
    Explorer,
    Researcher,
}

pub const SPEC_TIER_1: f32 = 100.0;
pub const SPEC_TIER_2: f32 = 300.0;
pub const SPEC_TIER_3: f32 = 600.0;

#[derive(Clone, Debug, Reflect)]
pub struct RegionState {
    pub region_id: RegionId,
    pub specialization: Option<SpecializationType>,
    pub target_specialization: Option<SpecializationType>,
    pub nutrients: f32,
    pub energy: f32,
    pub biomass: f32,
    pub specialization_investment: f32,
    pub tile_count: u32,
}

impl RegionState {
    pub fn new(id: RegionId) -> Self {
        Self {
            region_id: id,
            specialization: None,
            target_specialization: None,
            nutrients: 10.0,
            energy: 0.0,
            biomass: 0.0,
            specialization_investment: 0.0,
            tile_count: 0,
        }
    }

    pub fn tier(&self) -> u8 {
        if self.specialization_investment >= SPEC_TIER_3 {
            3
        } else if self.specialization_investment >= SPEC_TIER_2 {
            2
        } else if self.specialization_investment >= SPEC_TIER_1 {
            1
        } else {
            0
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

#[derive(Component, Clone, Debug, Reflect)]
pub struct HyphalTip {
    pub region_id: RegionId,
    pub age: u32,
}

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
    pub column_top: IVec2,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct MushroomEntity {
    pub fragment_id: FragmentId,
    pub pos: IVec2,
    pub vision_radius: f32,
}

#[derive(Clone, Debug, Reflect, PartialEq)]
pub struct ActiveAbility {
    pub name: String,
    pub energy_cost: f32,
    pub cooldown_max: u32,
    pub cooldown_remaining: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum UnlockPool {
    Organic,
    Mineral,
    Ruins,
    Decomposition,
}

#[derive(Clone, Debug, Reflect)]
pub struct UnlockOption {
    pub name: String,
    pub description: String,
    pub pool: UnlockPool,
}

#[derive(Resource, Default, Debug, Reflect)]
pub struct GameState {
    pub turn: u32,
    pub paused: bool,
    pub fragments_total: u32,
    pub fragments_fused: u32,
    pub mushrooms_fruited: u32,
    pub mushrooms_required: u32,
}

impl GameState {
    pub fn victory(&self) -> bool {
        self.fragments_fused >= self.fragments_total
            && self.mushrooms_fruited >= self.mushrooms_required
            && self.fragments_total > 0
    }
}

#[derive(Message)]
pub struct TurnAdvanced;

#[derive(Message)]
pub struct TileDiscovered {
    pub pos: IVec2,
    pub contents: Option<TileContents>,
}

#[derive(Message)]
pub struct StudyComplete {
    pub pos: IVec2,
    pub pool: UnlockPool,
}

#[derive(Message)]
pub struct DecompositionComplete {
    pub pos: IVec2,
}

#[derive(Message)]
pub struct FragmentFused {
    pub fragment_id: FragmentId,
}

#[derive(Message)]
pub struct SlotMachineTriggered {
    pub pool: UnlockPool,
    pub options: Vec<UnlockOption>,
}

pub const BIOMASS_FLIP_RATIO: f32 = 1.5;
pub const ANASTOMOSIS_BIOMASS_BONUS: f32 = 0.5;
pub const MUSHROOM_MOISTURE_BONUS: f32 = 0.2;
pub const MUSHROOM_MOISTURE_RADIUS: i32 = 5;
pub const SPORE_RELAY_ACCURACY_RADIUS: i32 = 5;
pub const BACTERIA_BIOMASS_BLOCK_THRESHOLD: f32 = 5.0;
pub const TRADE_LINK_NEGLECT_LIMIT: u32 = 20;
