pub const BIOMASS_FLIP_RATIO: f32 = 1.5;
pub const MUSHROOM_MOISTURE_BONUS: f32 = 0.2;
pub const MUSHROOM_MOISTURE_RADIUS: i32 = 5;
pub const SPORE_RELAY_ACCURACY_RADIUS: i32 = 5;
pub const BACTERIA_BIOMASS_BLOCK_THRESHOLD: f32 = 5.0;
pub const TRADE_LINK_NEGLECT_LIMIT: u32 = 20;

pub const CLAIM_THRESHOLD: f32 = 0.3;
pub const HUB_THRESHOLD: f32 = 1.0;
pub const BIOMASS_CAP: f32 = 2.0;
pub const MIN_FLOW_DENSITY: f32 = 0.05;
pub const AUTONOMOUS_FLOW_WEIGHT: f32 = 0.1;
pub const BIASED_FLOW_WEIGHT: f32 = 0.6;
pub const GRADIENT_FLOW_WEIGHT: f32 = 0.1;
pub const FLOW_NOISE: f32 = 0.15;
pub const WATER_GROWTH_COST: f32 = 0.05;
pub const MAX_OUTFLOW_FRACTION: f32 = 0.1;
pub const MOISTURE_DIFFUSION_RATE: f32 = 0.05;
const _: () = assert!(WATER_GROWTH_COST > 0.0);

pub const BIAS_DECAY: f32 = 0.95;
pub const BIAS_STROKE_INTENSITY: f32 = 0.5;
pub const BIAS_MAGNITUDE_CAP: f32 = 1.5;
pub const DIEBACK_THRESHOLD: f32 = 0.05;
pub const DIEBACK_RATE: f32 = 0.95;
// Small enough that snap-to-zero hides float drift, large enough to actually fire.
pub const BIOMASS_SNAP_EPSILON: f32 = 0.001;

pub const DECOMP_RATE: f32 = 0.02;
pub const SUGAR_FROM_DECOMP: f32 = 0.5;
pub const SUGAR_FROM_SYMBIOSIS: f32 = 0.1;
pub const MELANIN_FROM_RADIATION: f32 = 0.1;
pub const RADIATION_DEPLETION_RATE: f32 = 0.1;
pub const MIN_TRADE_MOISTURE: f32 = 0.3;
pub const MOISTURE_COST_PER_SUGAR: f32 = 0.3;

pub const DRAG_THRESHOLD_PX: f32 = 6.0;
pub const TAP_TIME_SECS: f32 = 0.150;
pub const SAMPLE_INTERVAL_SECS: f32 = 0.050;
pub const SAMPLE_HEX_DISTANCE: f32 = 0.5;
pub const WISP_SENSE_RADIUS_HEX: u32 = 5;

pub const BIAS_GLOW_VISIBLE_THRESHOLD: f32 = 0.05;

pub const HIVE_PRODUCTION_SUGAR_COST: f32 = 1.0;
pub const HIVE_PRODUCTION_RATE: f32 = 0.05;
pub const UNIT_UPKEEP_SUGAR: f32 = 0.1;
pub const UNIT_CAP_BASE: u32 = 2;
pub const UNIT_CAP_PER_HIVE: u32 = 2;

pub const UNIT_SPEED_HEXES_PER_SEC: f32 = 1.0;

/// Minimum hex distance from any owned tile to a valid founding site.
pub const MIN_FOUNDING_DISTANCE: u32 = 6;
pub const FOUNDER_SEED_BIOMASS: f32 = 1.0;
pub const FOUNDER_SEED_SUGARS: f32 = 10.0;
