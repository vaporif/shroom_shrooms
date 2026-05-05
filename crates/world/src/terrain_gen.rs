use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use fungai_core::{
    BacteriaColonyAgent, FragmentAgent, FragmentId, GameState, GridPos, GridWorld, HyphalTip,
    LaunchConfig, NeutralFungusAgent, Occupant, PlantRootAgent, RegionId, RegionStates, RivalId,
    SpecializationType, TerrainType, Tile, TileContents,
};
use hexx::{Hex, HexOrientation, OffsetHexMode};
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 60;

const ROCK_PROB: f32 = 0.08;
const WATER_PROB: f32 = 0.04;
const ROOT_PROB: f32 = 0.03;
const RUIN_PROB: f32 = 0.02;
const TOXIC_PROB: f32 = 0.01;

const BACTERIA_SPREAD_INTERVAL: u32 = 10;

/// Converts offset grid coordinates (col, row) to axial hex coordinates.
/// Uses pointy-top orientation with odd-row offset, matching the project's hex layout.
fn offset_to_hex(col: i32, row: i32) -> Hex {
    Hex::from_offset_coordinates([col, row], OffsetHexMode::Odd, HexOrientation::Pointy)
}

#[derive(Clone, Copy)]
struct TileBase {
    terrain: TerrainType,
    moisture: f32,
    nutrient_level: f32,
}

#[derive(Default)]
struct Placements {
    contents: HashMap<Hex, TileContents>,
    fragments: Vec<(Hex, FragmentId)>,
    fungi: Vec<(Hex, u32)>,
    plants: Vec<(Hex, u32)>,
    bacteria: Vec<Hex>,
}

pub fn terrain_generation(
    mut commands: Commands,
    mut grid: ResMut<GridWorld>,
    mut game_state: ResMut<GameState>,
    mut region_states: ResMut<RegionStates>,
    config: Res<LaunchConfig>,
) {
    let mut rng = StdRng::seed_from_u64(config.seed);
    grid.width = MAP_WIDTH;
    grid.height = MAP_HEIGHT;

    let mut tile_data = build_tile_data(&mut rng);
    let mut soil_pool = build_soil_pool(&tile_data, &mut rng);
    let mut placements = place_features(&mut rng, &mut tile_data, &mut soil_pool, &mut game_state);

    let player_rid = init_player_region(&mut region_states);
    let player_start = offset_to_hex(MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let player_hexes: HashSet<Hex> = player_start.range(2).collect();
    let rival_id = RivalId(0);
    let rival_start = offset_to_hex(MAP_WIDTH / 4, MAP_HEIGHT / 4);
    let rival_hexes: HashSet<Hex> = rival_start.range(1).collect();

    spawn_world_tiles(
        &mut commands,
        &mut grid,
        &tile_data,
        &mut placements,
        player_rid,
        rival_id,
        &player_hexes,
        &rival_hexes,
    );
    spawn_agents(&mut commands, placements);
    spawn_initial_tips(&mut commands, &grid, player_start, player_rid);
}

fn pick_terrain(rng: &mut StdRng, y: i32, depth_ratio: f32) -> TerrainType {
    if y == MAP_HEIGHT - 1 {
        return TerrainType::Surface;
    }
    if rng.random::<f32>() < ROCK_PROB * depth_ratio {
        return TerrainType::Rock;
    }
    if rng.random::<f32>() < WATER_PROB {
        return TerrainType::Water;
    }
    if y > MAP_HEIGHT / 2 && rng.random::<f32>() < ROOT_PROB {
        return TerrainType::Root;
    }
    if rng.random::<f32>() < RUIN_PROB * depth_ratio {
        return TerrainType::Ruin;
    }
    if rng.random::<f32>() < TOXIC_PROB * depth_ratio {
        return TerrainType::Toxic;
    }
    TerrainType::Soil
}

fn build_tile_data(rng: &mut StdRng) -> HashMap<Hex, TileBase> {
    let mut data = HashMap::with_capacity((MAP_WIDTH * MAP_HEIGHT) as usize);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let hex = offset_to_hex(x, y);
            let depth_ratio = 1.0 - (y as f32 / MAP_HEIGHT as f32);
            let terrain = pick_terrain(rng, y, depth_ratio);
            let moisture = (0.3 + 0.5 * (y as f32 / MAP_HEIGHT as f32) + rng.random::<f32>() * 0.2)
                .clamp(0.0, 1.0);
            let nutrient_level = 0.2 + rng.random::<f32>() * 0.6;
            data.insert(
                hex,
                TileBase {
                    terrain,
                    moisture,
                    nutrient_level,
                },
            );
        }
    }
    data
}

/// Soil hexes available for feature placement, excluding map borders and the surface row.
fn build_soil_pool(tile_data: &HashMap<Hex, TileBase>, rng: &mut StdRng) -> Vec<Hex> {
    let mut pool = Vec::new();
    for y in 1..MAP_HEIGHT - 2 {
        for x in 1..MAP_WIDTH - 1 {
            let hex = offset_to_hex(x, y);
            if let Some(base) = tile_data.get(&hex)
                && base.terrain == TerrainType::Soil
            {
                pool.push(hex);
            }
        }
    }
    pool.shuffle(rng);
    pool
}

fn place_features(
    rng: &mut StdRng,
    tile_data: &mut HashMap<Hex, TileBase>,
    soil_pool: &mut Vec<Hex>,
    game_state: &mut GameState,
) -> Placements {
    let mut p = Placements::default();

    let fragment_count = rng.random_range(3u32..=5);
    game_state.fragments_total = fragment_count;
    game_state.mushrooms_required = fragment_count;
    for i in 0..fragment_count {
        let Some(pos) = pop_unclaimed(soil_pool, &p.contents) else {
            break;
        };
        p.contents
            .insert(pos, TileContents::Fragment(FragmentId(i)));
        p.fragments.push((pos, FragmentId(i)));
    }

    for i in 0..rng.random_range(3u32..=5) {
        let Some(pos) = pop_unclaimed(soil_pool, &p.contents) else {
            break;
        };
        p.contents.insert(pos, TileContents::UniqueDecomposable(i));
    }

    for i in 0..rng.random_range(2u32..=4) {
        let Some(pos) = pop_unclaimed(soil_pool, &p.contents) else {
            break;
        };
        p.contents.insert(pos, TileContents::NeutralFungus(i));
        p.fungi.push((pos, i));
    }

    // Plants need proximity to surface; force terrain to Root regardless of base type.
    for i in 0..rng.random_range(3u32..=6) {
        let x = rng.random_range(0..MAP_WIDTH);
        let y = rng.random_range(MAP_HEIGHT / 2..MAP_HEIGHT - 1);
        let pos = offset_to_hex(x, y);
        if p.contents.contains_key(&pos) {
            continue;
        }
        p.contents.insert(pos, TileContents::PlantRoot(i));
        if let Some(base) = tile_data.get_mut(&pos) {
            base.terrain = TerrainType::Root;
        }
        p.plants.push((pos, i));
    }

    for _ in 0..rng.random_range(1u32..=2) {
        let Some(pos) = pop_unclaimed(soil_pool, &p.contents) else {
            break;
        };
        p.bacteria.push(pos);
    }

    p
}

fn pop_unclaimed(pool: &mut Vec<Hex>, claimed: &HashMap<Hex, TileContents>) -> Option<Hex> {
    while let Some(pos) = pool.pop() {
        if !claimed.contains_key(&pos) {
            return Some(pos);
        }
    }
    None
}

fn init_player_region(region_states: &mut RegionStates) -> RegionId {
    let rid = region_states.create_region();
    if let Some(state) = region_states.get_mut(rid) {
        state.nutrients = 100.0;
        state.energy = 20.0;
        state.specialization = Some(SpecializationType::Decomposer);
        state.target_specialization = Some(SpecializationType::Decomposer);
    }
    rid
}

#[allow(clippy::too_many_arguments)]
fn spawn_world_tiles(
    commands: &mut Commands,
    grid: &mut GridWorld,
    tile_data: &HashMap<Hex, TileBase>,
    placements: &mut Placements,
    player_rid: RegionId,
    rival_id: RivalId,
    player_hexes: &HashSet<Hex>,
    rival_hexes: &HashSet<Hex>,
) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let hex = offset_to_hex(x, y);
            let base = tile_data[&hex];
            let tile = if player_hexes.contains(&hex) {
                Tile {
                    terrain: TerrainType::Soil,
                    occupant: Occupant::Player(player_rid),
                    nutrient_level: 0.8,
                    discovered: true,
                    biomass: 1.0,
                    ..default()
                }
            } else if rival_hexes.contains(&hex) {
                Tile {
                    terrain: TerrainType::Soil,
                    occupant: Occupant::Rival(rival_id),
                    biomass: 1.5,
                    ..default()
                }
            } else {
                Tile {
                    terrain: base.terrain,
                    nutrient_level: base.nutrient_level,
                    moisture: base.moisture,
                    contents: placements.contents.remove(&hex),
                    ..default()
                }
            };
            let entity = commands.spawn((GridPos(hex), tile)).id();
            grid.tiles.insert(hex, entity);
        }
    }
}

fn spawn_agents(commands: &mut Commands, p: Placements) {
    for (pos, fid) in p.fragments {
        commands.spawn((
            GridPos(pos),
            FragmentAgent {
                fragment_id: fid,
                fused: false,
            },
        ));
    }
    for (pos, fungus_id) in p.fungi {
        commands.spawn((
            GridPos(pos),
            NeutralFungusAgent {
                fungus_id,
                merge_progress: 0.0,
            },
        ));
    }
    for (pos, plant_id) in p.plants {
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
    for pos in p.bacteria {
        commands.spawn((
            GridPos(pos),
            BacteriaColonyAgent {
                spread_timer: 0,
                spread_interval: BACTERIA_SPREAD_INTERVAL,
            },
        ));
    }
}

fn spawn_initial_tips(
    commands: &mut Commands,
    grid: &GridWorld,
    player_start: Hex,
    player_rid: RegionId,
) {
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
        app.insert_resource(LaunchConfig { seed: 12345 });
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
