use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use fungai_core::{
    GridPos, GridWorld, Hex, HexLayout, HexOrientation, OffsetHexMode, TerrainType, Tile,
};

use crate::data_layer::DiscoveryMap;

const MAP_WIDTH: u32 = 80;
const MAP_HEIGHT: u32 = 60;

const ATLAS_PATH: &str = "sprites/terrain/terrain_atlas.png";

// Visual cell size in the atlas PNG (rounded up to whole pixels for the
// generator). This is what `bevy_ecs_tilemap` samples per tile.
const TILE_PX_W: f32 = 49.0; // matches the generator's TILE_W
const TILE_PX_H: f32 = 56.0;

// Spacing between tile centers in world space. Must equal hexx's column /
// row strides for `scale = 28` pointy-top so other rendering layers (network
// splines, organism sprites, region highlights) align with the terrain
// without per-column drift. Drift would otherwise compound to ~40px across
// the 80-column grid (TILE_PX_W - 28*sqrt(3) ≈ 0.5px per column).
const GRID_PX_W: f32 = 28.0 * 1.732_050_8;
const GRID_PX_H: f32 = 56.0;

/// One row per `TerrainType` variant. Asserted at runtime against the loaded
/// atlas in `assert_atlas_addresses_all_terrains`.
const REQUIRED_TERRAIN_INDICES: u32 = 7;

const TERRAIN_Z: f32 = -10.0;

const VISIBLE: LinearRgba = LinearRgba::new(1.0, 1.0, 1.0, 1.0);
const HIDDEN: LinearRgba = LinearRgba::new(0.18, 0.18, 0.22, 1.0);

pub fn terrain_base_color(terrain: TerrainType) -> LinearRgba {
    match terrain {
        TerrainType::Soil => LinearRgba::new(0.18, 0.12, 0.07, 1.0),
        TerrainType::Rock => LinearRgba::new(0.20, 0.20, 0.22, 1.0),
        TerrainType::Water => LinearRgba::new(0.06, 0.12, 0.30, 1.0),
        TerrainType::Root => LinearRgba::new(0.10, 0.18, 0.08, 1.0),
        TerrainType::Ruin => LinearRgba::new(0.22, 0.20, 0.14, 1.0),
        TerrainType::Toxic => LinearRgba::new(0.18, 0.28, 0.05, 1.0),
        TerrainType::Surface => LinearRgba::new(0.10, 0.22, 0.10, 1.0),
    }
}

pub fn terrain_type_index(terrain: TerrainType) -> u32 {
    match terrain {
        TerrainType::Soil => 0,
        TerrainType::Rock => 1,
        TerrainType::Water => 2,
        TerrainType::Root => 3,
        TerrainType::Ruin => 4,
        TerrainType::Toxic => 5,
        TerrainType::Surface => 6,
    }
}

/// Converts a `hexx` axial coordinate into a `bevy_ecs_tilemap` `TilePos`,
/// preserving `OffsetHexMode::Odd` parity by routing through `to_offset_coordinates`.
/// Returns `None` if the offset coordinates are negative — `TilePos` is `u32`-indexed
/// and a wrapping cast would point at a phantom cell far outside the grid.
pub fn hex_to_tile_pos(hex: Hex) -> Option<TilePos> {
    let [col, row] = hex.to_offset_coordinates(OffsetHexMode::Odd, HexOrientation::Pointy);
    if col < 0 || row < 0 {
        return None;
    }
    Some(TilePos {
        x: col as u32,
        y: row as u32,
    })
}

fn discovery_color(level: f32) -> Color {
    LinearRgba {
        red: HIDDEN.red + (VISIBLE.red - HIDDEN.red) * level,
        green: HIDDEN.green + (VISIBLE.green - HIDDEN.green) * level,
        blue: HIDDEN.blue + (VISIBLE.blue - HIDDEN.blue) * level,
        alpha: 1.0,
    }
    .into()
}

/// Holds the atlas handle so `assert_atlas_addresses_all_terrains` can re-read
/// the image once Bevy's async loader has populated `Assets<Image>`. Cleared
/// after the assertion fires.
#[derive(Resource, Default)]
pub struct PendingAtlasCheck(pub Option<Handle<Image>>);

pub fn spawn_terrain_tilemap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    grid: Res<GridWorld>,
    layout: Res<HexLayout>,
    discovery: Res<DiscoveryMap>,
    tiles: Query<&Tile>,
    mut pending: ResMut<PendingAtlasCheck>,
) {
    let texture: Handle<Image> = asset_server.load(ATLAS_PATH);
    pending.0 = Some(texture.clone());

    let map_size = TilemapSize {
        x: MAP_WIDTH,
        y: MAP_HEIGHT,
    };
    let tile_size = TilemapTileSize {
        x: TILE_PX_W,
        y: TILE_PX_H,
    };
    let grid_size = TilemapGridSize {
        x: GRID_PX_W,
        y: GRID_PX_H,
    };
    let map_type = TilemapType::Hexagon(HexCoordSystem::RowOdd);

    let tilemap_entity = commands.spawn_empty().id();
    let mut storage = TileStorage::empty(map_size);

    // Decorate each existing simulation entity with a TileBundle. Hexes whose
    // offset coordinates fall outside the rectangular tilemap bounds (e.g.
    // negative offsets near the origin in some test grids) are skipped:
    // TileStorage::set indexes a flat Vec and would multiply-overflow on
    // wrapped u32 values.
    for (&hex, &entity) in grid.tiles.iter() {
        let Ok(tile) = tiles.get(entity) else {
            continue;
        };
        let Some(tp) = hex_to_tile_pos(hex) else {
            continue;
        };
        if tp.x >= map_size.x || tp.y >= map_size.y {
            continue;
        }
        let level = discovery.discovered.get(&hex).copied().unwrap_or(0.0);
        commands.entity(entity).insert(TileBundle {
            position: tp,
            tilemap_id: TilemapId(tilemap_entity),
            texture_index: TileTextureIndex(terrain_type_index(tile.terrain)),
            color: TileColor(discovery_color(level)),
            ..Default::default()
        });
        storage.set(&tp, entity);
    }

    // Compute the offset that aligns tilemap world space with hexx world space.
    // tile_pos.center_in_world is in tilemap-local space; layout.hex_to_world_pos
    // is the engine-wide truth. We translate the tilemap so they agree at H=0.
    let zero_tp = hex_to_tile_pos(Hex::ZERO).expect("Hex::ZERO is in-bounds");
    let local = zero_tp.center_in_world(
        &map_size,
        &grid_size,
        &tile_size,
        &map_type,
        &TilemapAnchor::None,
    );
    let world = layout.hex_to_world_pos(Hex::ZERO);
    let origin = Vec3::new(world.x - local.x, world.y - local.y, TERRAIN_Z);

    commands.entity(tilemap_entity).insert(TilemapBundle {
        size: map_size,
        storage,
        texture: TilemapTexture::Single(texture),
        tile_size,
        grid_size,
        map_type,
        anchor: TilemapAnchor::None,
        transform: Transform::from_translation(origin),
        ..Default::default()
    });
}

// `ParamSet` is the canonical way to overlap two `&mut TileColor` queries —
// see plan note. The combined type is unavoidably wide, so silence the lint
// at the call site rather than in `Cargo.toml` where it would also affect
// other systems we haven't yet written.
#[allow(clippy::type_complexity)]
pub fn terrain_tile_update_system(
    mut sets: ParamSet<(
        Query<(&Tile, &GridPos, &mut TileTextureIndex, &mut TileColor), Changed<Tile>>,
        Query<(&GridPos, &mut TileColor)>,
    )>,
    discovery: Res<DiscoveryMap>,
    untiled: Query<Entity, (With<Tile>, Without<TilePos>)>,
    mut warned_untiled: Local<bool>,
) {
    // Spec §"Tilemap ↔ simulation desync": warn exactly once if a tile entity
    // lacks a TilePos. Don't spam: a stale spawn loop could otherwise emit
    // thousands of warnings per frame.
    if !*warned_untiled && let Some(entity) = untiled.iter().next() {
        warn!(
            "terrain_tile_update_system: entity {entity:?} has Tile but no TilePos -- \
             spawn_terrain_tilemap likely ran before terrain_generation populated GridWorld"
        );
        *warned_untiled = true;
    }

    // Path 1: per-changed-tile texture index + color refresh.
    for (tile, gpos, mut idx, mut color) in &mut sets.p0() {
        idx.0 = terrain_type_index(tile.terrain);
        let level = discovery.discovered.get(&gpos.0).copied().unwrap_or(0.0);
        color.0 = discovery_color(level);
    }

    // Path 2: discovery sweep, exactly once per sim tick when DiscoveryMap mutates.
    if discovery.is_changed() {
        for (gpos, mut color) in &mut sets.p1() {
            let level = discovery.discovered.get(&gpos.0).copied().unwrap_or(0.0);
            color.0 = discovery_color(level);
        }
    }
}

/// Spec §"Asset loading": once the atlas image lands in `Assets<Image>`,
/// verify it can address all `REQUIRED_TERRAIN_INDICES` indices and panic
/// loudly if it cannot. Asset loads are async, so this runs every Update
/// until the handle resolves; clears the pending handle on success.
pub fn assert_atlas_addresses_all_terrains(
    mut pending: ResMut<PendingAtlasCheck>,
    images: Res<Assets<Image>>,
) {
    let Some(handle) = pending.0.as_ref() else {
        return;
    };
    let Some(image) = images.get(handle) else {
        return;
    };
    let w = image.texture_descriptor.size.width;
    let h = image.texture_descriptor.size.height;
    let cols = w / TILE_PX_W as u32;
    let rows = h / TILE_PX_H as u32;
    let addressable = cols.saturating_mul(rows);
    assert!(
        addressable >= REQUIRED_TERRAIN_INDICES,
        "terrain atlas is too small: {w}x{h} px / {tw}x{th} tile = {addressable} indices, \
         need at least {req} for all TerrainType variants",
        tw = TILE_PX_W as u32,
        th = TILE_PX_H as u32,
        req = REQUIRED_TERRAIN_INDICES,
    );
    pending.0 = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_base_color_returns_dark_palette() {
        let soil = terrain_base_color(TerrainType::Soil);
        assert!(soil.red < 0.25);
        assert!(soil.green < 0.20);
        assert!(soil.blue < 0.15);

        let water = terrain_base_color(TerrainType::Water);
        assert!(water.blue > water.red);
        assert!(water.blue > water.green);
    }
}

#[cfg(test)]
mod tilemap_tests {
    use super::*;
    use bevy::MinimalPlugins;
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use fungai_core::{
        GridPos, GridWorld, Hex, HexOrientation, OffsetHexMode, TerrainType, Tile,
        create_hex_layout,
    };
    // NOTE: `super::*` exposes only items actually defined in `terrain_render.rs`;
    // names brought in via `use fungai_core::{...}` at the top of that file are
    // private and do NOT leak through `super::*`. List every type used below
    // explicitly.

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ImagePlugin::default(),
        ));
        // TilemapPlugin pulls in `TilemapRenderingPlugin`, which calls
        // `app.sub_app_mut(RenderApp)` and panics in headless tests. Our tests
        // only mutate Tile* components directly and read tilemap-component
        // values back; they don't need the render pipeline. Skip the plugin.
        app.init_resource::<GridWorld>();
        app.init_resource::<crate::data_layer::DiscoveryMap>();
        app.init_resource::<PendingAtlasCheck>();
        app.insert_resource(create_hex_layout());
        // Deliberately do NOT register `extract_discovery_map`: it calls
        // `discovered.clear()` every Update tick (data_layer.rs), which would
        // erase the manual inserts these tests rely on. The system under test
        // (`terrain_tile_update_system`) reads `Res<DiscoveryMap>` directly, so
        // mutating the resource via `resource_mut` is sufficient to flip its
        // change tick and drive Path 2.
        app
    }

    fn spawn_grid_tile(app: &mut App, hex: Hex, terrain: TerrainType) -> Entity {
        let e = app
            .world_mut()
            .spawn((
                GridPos(hex),
                Tile {
                    terrain,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(hex, e);
        e
    }

    #[test]
    fn hex_to_tile_pos_round_trips() {
        // Origin (== from_offset([0, 0])), the centre, and the four corners of
        // the 80x60 grid. `Hex::ZERO` and `from_offset([0, 0], Odd, Pointy)`
        // collapse to the same axial value, so only one of them is included.
        let samples = [
            Hex::ZERO,
            Hex::from_offset_coordinates([40, 30], OffsetHexMode::Odd, HexOrientation::Pointy),
            Hex::from_offset_coordinates([79, 0], OffsetHexMode::Odd, HexOrientation::Pointy),
            Hex::from_offset_coordinates([0, 59], OffsetHexMode::Odd, HexOrientation::Pointy),
            Hex::from_offset_coordinates([79, 59], OffsetHexMode::Odd, HexOrientation::Pointy),
        ];
        for h in samples {
            let tp = hex_to_tile_pos(h).expect("test hex must be in-bounds");
            let back = Hex::from_offset_coordinates(
                [tp.x as i32, tp.y as i32],
                OffsetHexMode::Odd,
                HexOrientation::Pointy,
            );
            assert_eq!(back, h, "round-trip failed for {h:?}");
        }
    }

    #[test]
    fn tilemap_spawns_tile_for_each_hex() {
        let mut app = test_app();
        // All four positions must produce non-negative offset coordinates so
        // they fit the rectangular 80x60 TileStorage. Hex::new(2, -1) (which
        // the plan originally listed) maps to offset row=-1 and overflows the
        // u32 cast inside TileStorage::get; the diagonal Hex::new(2, 1) gives
        // offset (col=2, row=1) and exercises the same "non-axis-aligned"
        // case without going out of bounds.
        let positions = [Hex::ZERO, Hex::new(1, 0), Hex::new(0, 1), Hex::new(2, 1)];
        for &p in &positions {
            spawn_grid_tile(&mut app, p, TerrainType::Soil);
        }

        app.add_systems(Startup, spawn_terrain_tilemap);
        app.update();

        // Exactly one TileStorage on the tilemap entity, populated for every hex.
        let mut q = app.world_mut().query::<&TileStorage>();
        let storage = q.iter(app.world()).next().expect("TileStorage exists");
        let mut found = 0;
        for &p in &positions {
            let tp = hex_to_tile_pos(p).expect("test hex must be in-bounds");
            if storage.get(&tp).is_some() {
                found += 1;
            }
        }
        assert_eq!(
            found,
            positions.len(),
            "every hex should appear in TileStorage"
        );
    }

    #[test]
    fn terrain_tile_update_changes_texture_index() {
        let mut app = test_app();
        let pos = Hex::new(2, 3);
        let entity = spawn_grid_tile(&mut app, pos, TerrainType::Soil);

        app.add_systems(Startup, spawn_terrain_tilemap);
        app.add_systems(PostUpdate, terrain_tile_update_system);
        app.update();

        // Flip terrain → the next update should mutate TileTextureIndex.
        app.world_mut().get_mut::<Tile>(entity).unwrap().terrain = TerrainType::Rock;
        app.update();

        let idx = app
            .world()
            .get::<TileTextureIndex>(entity)
            .expect("tile has index");
        assert_eq!(idx.0, terrain_type_index(TerrainType::Rock));
    }

    #[test]
    fn discovery_drives_tile_color() {
        let mut app = test_app();
        let pos = Hex::new(4, 4);
        let entity = spawn_grid_tile(&mut app, pos, TerrainType::Soil);

        app.add_systems(Startup, spawn_terrain_tilemap);
        app.add_systems(PostUpdate, terrain_tile_update_system);
        app.update();

        let dark = app
            .world()
            .get::<TileColor>(entity)
            .copied()
            .expect("color exists");

        app.world_mut()
            .resource_mut::<crate::data_layer::DiscoveryMap>()
            .discovered
            .insert(pos, 1.0);
        app.update();

        let lit = app
            .world()
            .get::<TileColor>(entity)
            .copied()
            .expect("color exists");
        let dark_rgba: Color = dark.0;
        let lit_rgba: Color = lit.0;
        assert!(
            lit_rgba.to_linear().red > dark_rgba.to_linear().red,
            "discovered tile should be brighter"
        );
    }

    #[test]
    fn tilemap_world_pos_aligns_with_hex_layout() {
        let mut app = test_app();
        let layout = create_hex_layout();
        // The plan originally listed Hex::new(-1, 0) and Hex::new(0, -1)
        // among the canonical samples, but both produce offset coordinates
        // with negative components. `hex_to_tile_pos` casts those to u32 and
        // `tp.center_in_world` then computes nonsense world positions.
        // Production never sees negative-offset hexes (the 80x60 grid is
        // strictly non-negative), so checking alignment only on in-bounds
        // hexes here matches the geometry the renderer actually serves.
        let canonical = [
            Hex::ZERO,
            Hex::new(1, 0),
            Hex::new(0, 1),
            Hex::from_offset_coordinates([0, 0], OffsetHexMode::Odd, HexOrientation::Pointy),
            Hex::from_offset_coordinates([79, 0], OffsetHexMode::Odd, HexOrientation::Pointy),
            Hex::from_offset_coordinates([0, 59], OffsetHexMode::Odd, HexOrientation::Pointy),
            Hex::from_offset_coordinates([79, 59], OffsetHexMode::Odd, HexOrientation::Pointy),
        ];
        for &h in &canonical {
            spawn_grid_tile(&mut app, h, TerrainType::Soil);
        }

        app.add_systems(Startup, spawn_terrain_tilemap);
        app.update();

        let (tilemap_transform, tilemap_size, grid_size, tile_size, map_type) = {
            let mut q = app.world_mut().query::<(
                &Transform,
                &TilemapSize,
                &TilemapGridSize,
                &TilemapTileSize,
                &TilemapType,
            )>();
            let (t, s, g, ts, m) = q.iter(app.world()).next().expect("tilemap exists");
            (*t, *s, *g, *ts, *m)
        };

        for &h in &canonical {
            let expected = layout.hex_to_world_pos(h);
            let tp = hex_to_tile_pos(h).expect("canonical hex must be in-bounds");
            let local = tp.center_in_world(
                &tilemap_size,
                &grid_size,
                &tile_size,
                &map_type,
                &TilemapAnchor::None,
            );
            let actual = tilemap_transform.translation.truncate() + local;
            let diff = (actual - expected).length();
            assert!(
                diff < 1.0,
                "hex {h:?} drifts by {diff}px (expected={expected:?}, actual={actual:?})"
            );
        }
    }

    #[test]
    fn discovery_applies_to_correct_neighbors() {
        // Even-row vs odd-row neighbour parity. With OffsetHexMode::Odd +
        // HexCoordSystem::RowOdd, lighting hex H must light H, not H's
        // row-shifted lookalike.
        let mut app = test_app();
        let target =
            Hex::from_offset_coordinates([5, 4], OffsetHexMode::Odd, HexOrientation::Pointy);
        let other =
            Hex::from_offset_coordinates([5, 5], OffsetHexMode::Odd, HexOrientation::Pointy);

        let target_e = spawn_grid_tile(&mut app, target, TerrainType::Soil);
        let other_e = spawn_grid_tile(&mut app, other, TerrainType::Soil);

        app.add_systems(Startup, spawn_terrain_tilemap);
        app.add_systems(PostUpdate, terrain_tile_update_system);
        app.update();

        let baseline_target = app.world().get::<TileColor>(target_e).copied().unwrap();
        let baseline_other = app.world().get::<TileColor>(other_e).copied().unwrap();

        app.world_mut()
            .resource_mut::<crate::data_layer::DiscoveryMap>()
            .discovered
            .insert(target, 1.0);
        app.update();

        let lit_target = app.world().get::<TileColor>(target_e).copied().unwrap();
        let lit_other = app.world().get::<TileColor>(other_e).copied().unwrap();

        assert!(
            lit_target.0.to_linear().red > baseline_target.0.to_linear().red,
            "target hex should brighten"
        );
        assert_eq!(
            lit_other.0.to_linear().red,
            baseline_other.0.to_linear().red,
            "neighbour with same offset col but different row must NOT brighten"
        );
    }
}
