use bevy::prelude::*;
use rand::RngExt;

/// Default map width when no `--width` flag is supplied.
pub const DEFAULT_MAP_WIDTH: i32 = 220;
/// Default map height when no `--height` flag is supplied.
pub const DEFAULT_MAP_HEIGHT: i32 = 120;
/// Tiles per hive, preserving the original 4800-tile / 6-hive density.
pub const TILES_PER_HIVE: u32 = 800;

#[derive(Resource, Clone, Debug, Reflect)]
pub struct LaunchConfig {
    pub seed: u64,
    pub width: i32,
    pub height: i32,
    pub hives: u32,
}

pub fn default_seed() -> u64 {
    if cfg!(debug_assertions) {
        420
    } else {
        rand::rng().random::<u64>()
    }
}

/// Area-scaled hive count for the given map dimensions, clamped to at least 1.
#[must_use]
pub fn default_hive_count(width: i32, height: i32) -> u32 {
    ((width * height) as u32 / TILES_PER_HIVE).max(1)
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            seed: default_seed(),
            width: DEFAULT_MAP_WIDTH,
            height: DEFAULT_MAP_HEIGHT,
            hives: default_hive_count(DEFAULT_MAP_WIDTH, DEFAULT_MAP_HEIGHT),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn launch_config_default_seed_is_420_in_debug() {
        assert_eq!(LaunchConfig::default().seed, 420);
    }

    #[test]
    fn launch_config_default_dimensions_are_220x120() {
        let config = LaunchConfig::default();
        assert_eq!(config.width, 220);
        assert_eq!(config.height, 120);
    }

    #[test]
    fn launch_config_default_hive_count_is_area_scaled() {
        // 220 * 120 = 26400 tiles / 800 = 33 hives.
        assert_eq!(LaunchConfig::default().hives, 33);
    }

    #[test]
    fn default_hive_count_clamps_to_at_least_one() {
        assert_eq!(default_hive_count(10, 10), 1);
    }
}
