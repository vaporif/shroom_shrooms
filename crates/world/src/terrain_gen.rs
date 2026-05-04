use std::collections::HashMap;

use bevy::prelude::*;
use fungai_core::{
    BacteriaColonyAgent, FragmentAgent, FragmentId, GameState, GridPos, GridWorld, HyphalTip,
    NeutralFungusAgent, Occupant, PlantRootAgent, RegionStates, RivalId, SpecializationType,
    TerrainType, Tile, TileContents,
};
use hexx::{Hex, HexOrientation, OffsetHexMode};
use rand::prelude::*;
use rand::rngs::StdRng;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 60;

/// Converts offset grid coordinates (col, row) to axial hex coordinates.
/// Uses pointy-top orientation with odd-row offset, matching the project's hex layout.
fn offset_to_hex(col: i32, row: i32) -> Hex {
    Hex::from_offset_coordinates([col, row], OffsetHexMode::Odd, HexOrientation::Pointy)
}

#[derive(Resource)]
pub struct TerrainSeed(pub u64);

impl Default for TerrainSeed {
    fn default() -> Self {
        Self(42)
    }
}

pub fn terrain_generation(
    mut commands: Commands,
    mut grid: ResMut<GridWorld>,
    mut game_state: ResMut<GameState>,
    mut region_states: ResMut<RegionStates>,
    seed: Res<TerrainSeed>,
) {
    let mut rng = StdRng::seed_from_u64(seed.0);
    grid.width = MAP_WIDTH;
    grid.height = MAP_HEIGHT;

    // Pass 1: precompute terrain, moisture, nutrient_level for every hex.
    let mut tile_data: HashMap<Hex, (TerrainType, f32, f32)> = HashMap::new();
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let hex = offset_to_hex(x, y);
            let depth_ratio = 1.0 - (y as f32 / MAP_HEIGHT as f32);
            let terrain = if y == MAP_HEIGHT - 1 {
                TerrainType::Surface
            } else if rng.random::<f32>() < 0.08 * depth_ratio {
                TerrainType::Rock
            } else if rng.random::<f32>() < 0.04 {
                TerrainType::Water
            } else if y > MAP_HEIGHT / 2 && rng.random::<f32>() < 0.03 {
                TerrainType::Root
            } else if rng.random::<f32>() < 0.02 * depth_ratio {
                TerrainType::Ruin
            } else if rng.random::<f32>() < 0.01 * depth_ratio {
                TerrainType::Toxic
            } else {
                TerrainType::Soil
            };
            let moisture = (0.3 + 0.5 * (y as f32 / MAP_HEIGHT as f32) + rng.random::<f32>() * 0.2)
                .clamp(0.0, 1.0);
            let nutrient_level = 0.2 + rng.random::<f32>() * 0.6;
            tile_data.insert(hex, (terrain, moisture, nutrient_level));
        }
    }

    // Pass 2: pick placements. random_soil_pos_pre_spawn filters by terrain via tile_data
    // AND avoids hexes already claimed in `placements`.
    let mut placements: HashMap<Hex, TileContents> = HashMap::new();
    let mut terrain_overrides: HashMap<Hex, TerrainType> = HashMap::new();
    let mut fragment_spawns: Vec<(Hex, FragmentId)> = Vec::new();
    let mut fungus_spawns: Vec<(Hex, u32)> = Vec::new();
    let mut plant_spawns: Vec<(Hex, u32)> = Vec::new();
    let mut bacteria_spawns: Vec<Hex> = Vec::new();

    let fragment_count = rng.random_range(3u32..=5);
    game_state.fragments_total = fragment_count;
    game_state.mushrooms_required = fragment_count;
    for i in 0..fragment_count {
        let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
        placements.insert(pos, TileContents::Fragment(FragmentId(i)));
        fragment_spawns.push((pos, FragmentId(i)));
    }

    let decomp_count = rng.random_range(3u32..=5);
    for i in 0..decomp_count {
        let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
        placements.insert(pos, TileContents::UniqueDecomposable(i));
    }

    let fungi_count = rng.random_range(2u32..=4);
    for i in 0..fungi_count {
        let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
        placements.insert(pos, TileContents::NeutralFungus(i));
        fungus_spawns.push((pos, i));
    }

    // Roots need proximity to surface
    let plant_count = rng.random_range(3u32..=6);
    for i in 0..plant_count {
        let x = rng.random_range(0..MAP_WIDTH);
        let y = rng.random_range(MAP_HEIGHT / 2..MAP_HEIGHT - 1);
        let pos = offset_to_hex(x, y);
        if placements.contains_key(&pos) {
            continue;
        }
        placements.insert(pos, TileContents::PlantRoot(i));
        terrain_overrides.insert(pos, TerrainType::Root);
        plant_spawns.push((pos, i));
    }

    let bacteria_count = rng.random_range(1u32..=2);
    for _ in 0..bacteria_count {
        let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
        bacteria_spawns.push(pos);
    }

    // Player and rival start positions: derived deterministically, override entire Tile.
    let player_rid = region_states.create_region();
    if let Some(state) = region_states.get_mut(player_rid) {
        state.nutrients = 100.0;
        state.energy = 20.0;
        state.specialization = Some(SpecializationType::Decomposer);
        state.target_specialization = Some(SpecializationType::Decomposer);
    }
    let player_start = offset_to_hex(MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let player_hexes: Vec<Hex> = player_start.range(2).collect();
    let rival_id = RivalId(0);
    let rival_start = offset_to_hex(MAP_WIDTH / 4, MAP_HEIGHT / 4);
    let rival_hexes: Vec<Hex> = rival_start.range(1).collect();

    // Pass 3: spawn every tile in one go. Player/rival hexes get their full override.
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let hex = offset_to_hex(x, y);
            let (mut terrain, moisture, nutrient_level) = tile_data[&hex];
            if let Some(override_terrain) = terrain_overrides.get(&hex).copied() {
                terrain = override_terrain;
            }
            let tile = if player_hexes.contains(&hex) {
                Tile {
                    terrain: TerrainType::Soil,
                    occupant: Occupant::Player(player_rid),
                    nutrient_level: 0.8,
                    moisture: 0.5,
                    discovered: true,
                    contents: None,
                    biomass: 1.0,
                    nutrient_gradient: Vec2::ZERO,
                    priority_bias: Vec2::ZERO,
                }
            } else if rival_hexes.contains(&hex) {
                Tile {
                    terrain: TerrainType::Soil,
                    occupant: Occupant::Rival(rival_id),
                    nutrient_level: 0.5,
                    moisture: 0.5,
                    discovered: false,
                    contents: None,
                    biomass: 1.5,
                    nutrient_gradient: Vec2::ZERO,
                    priority_bias: Vec2::ZERO,
                }
            } else {
                Tile {
                    terrain,
                    nutrient_level,
                    moisture,
                    contents: placements.remove(&hex),
                    ..default()
                }
            };
            let entity = commands.spawn((GridPos(hex), tile)).id();
            grid.tiles.insert(hex, entity);
        }
    }

    // Pass 4: spawn agent entities (these are separate entities, not tile components).
    for (pos, fid) in fragment_spawns {
        commands.spawn((
            GridPos(pos),
            FragmentAgent {
                fragment_id: fid,
                fused: false,
            },
        ));
    }
    for (pos, fungus_id) in fungus_spawns {
        commands.spawn((
            GridPos(pos),
            NeutralFungusAgent {
                fungus_id,
                merge_progress: 0.0,
            },
        ));
    }
    for (pos, plant_id) in plant_spawns {
        commands.spawn((
            GridPos(pos),
            PlantRootAgent {
                plant_id,
                health: 1.0,
                trade_active: false,
                nutrient_intake: 0.0,
                sugar_output: 0.0,
                neglect_timer: 0,
            },
        ));
    }
    for pos in bacteria_spawns {
        commands.spawn((
            GridPos(pos),
            BacteriaColonyAgent {
                spread_timer: 0,
                spread_interval: 10,
            },
        ));
    }
    for neighbor in player_start.all_neighbors() {
        if grid.tiles.contains_key(&neighbor) {
            commands.spawn((
                GridPos(neighbor),
                HyphalTip {
                    region_id: player_rid,
                    age: 0,
                },
            ));
        }
    }
}

fn random_soil_pos_pre_spawn(
    tile_data: &HashMap<Hex, (TerrainType, f32, f32)>,
    rng: &mut StdRng,
    placements: &HashMap<Hex, TileContents>,
) -> Hex {
    loop {
        let x = rng.random_range(1..MAP_WIDTH - 1);
        let y = rng.random_range(1..MAP_HEIGHT - 2);
        let hex = offset_to_hex(x, y);
        if placements.contains_key(&hex) {
            continue;
        }
        if let Some((TerrainType::Soil, _, _)) = tile_data.get(&hex) {
            return hex;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fungai_core::RegionStates;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<GameState>();
        app.init_resource::<RegionStates>();
        app.insert_resource(TerrainSeed(12345));
        app
    }

    #[test]
    fn generates_grid_with_correct_dimensions() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        assert_eq!(grid.width, MAP_WIDTH);
        assert_eq!(grid.height, MAP_HEIGHT);
        assert_eq!(grid.tiles.len(), (MAP_WIDTH * MAP_HEIGHT) as usize);
    }

    #[test]
    fn top_row_is_surface_terrain() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        for x in 0..MAP_WIDTH {
            let hex = offset_to_hex(x, MAP_HEIGHT - 1);
            let entity = grid.tiles[&hex];
            let tile = app.world().get::<Tile>(entity).unwrap();
            assert_eq!(tile.terrain, TerrainType::Surface);
        }
    }

    #[test]
    fn places_fragments() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let mut fragment_count = 0u32;
        for tile in app.world_mut().query::<&Tile>().iter(app.world()) {
            if matches!(tile.contents, Some(TileContents::Fragment(_))) {
                fragment_count += 1;
            }
        }
        assert!((3..=5).contains(&fragment_count));
    }

    #[test]
    fn moisture_higher_near_surface() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let near_surface = offset_to_hex(0, MAP_HEIGHT - 2);
        let deep = offset_to_hex(0, 0);
        let surface_tile = app.world().get::<Tile>(grid.tiles[&near_surface]).unwrap();
        let deep_tile = app.world().get::<Tile>(grid.tiles[&deep]).unwrap();
        assert!(surface_tile.moisture > deep_tile.moisture);
    }

    #[test]
    fn fragment_tiles_preserve_rng_nutrient_and_moisture() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let mut fragment_count = 0;
        for tile in app.world_mut().query::<&Tile>().iter(app.world()) {
            if matches!(tile.contents, Some(TileContents::Fragment(_))) {
                fragment_count += 1;
                assert!(
                    (tile.nutrient_level - 0.5).abs() > f32::EPSILON
                        || (tile.moisture - 0.5).abs() > f32::EPSILON,
                    "fragment tile reset to Tile::default() — nutrient {} moisture {}",
                    tile.nutrient_level,
                    tile.moisture,
                );
            }
        }
        assert!(fragment_count > 0, "expected at least one fragment");
    }
}
