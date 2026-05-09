use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_ecs_tilemap::prelude::*;
use kingdom_core::{
    GridPos, GridWorld, Hex, HexLayout, HexOrientation, OffsetHexMode, TerrainType, Tile,
};

use crate::data_layer::DiscoveryMap;

// TODO: to change
const MAP_WIDTH: u32 = 80;
const MAP_HEIGHT: u32 = 60;

// Rendered tile size in world units. Larger than GRID_PX_W/H so adjacent tile
// sprites overlap by ~5px and hex-edge AA seams don't show through.
const TILE_PX_W: f32 = 54.0;
const TILE_PX_H: f32 = 60.0;

// Spacing between tile centers in world space. Must match hexx's column / row
// strides for scale=28 pointy-top so other layers (splines, sprites, highlights)
// don't drift relative to the terrain — the gap is ~0.5px per column, which
// would compound to ~40px across the 80-column grid.
const GRID_PX_W: f32 = 28.0 * 1.732_050_8;
const GRID_PX_H: f32 = 56.0;

const TERRAIN_TYPES: [TerrainType; 7] = [
    TerrainType::Soil,
    TerrainType::Rock,
    TerrainType::Water,
    TerrainType::Root,
    TerrainType::Ruin,
    TerrainType::Toxic,
    TerrainType::Surface,
];

const TERRAIN_Z: f32 = -10.0;

const LIT_TOP: LinearRgba = LinearRgba::new(1.05, 0.95, 0.78, 1.0);
const LIT_BOTTOM: LinearRgba = LinearRgba::new(0.55, 0.62, 0.85, 1.0);
const HIDDEN: LinearRgba = LinearRgba::new(0.0, 0.0, 0.0, 1.0);

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

/// Convert a `hexx` axial coordinate to a `bevy_ecs_tilemap` `TilePos`,
/// keeping `OffsetHexMode::Odd` parity via `to_offset_coordinates`.
/// Returns `None` for negative offsets: `TilePos` is `u32`-indexed and a
/// wrapping cast would point at a phantom cell far outside the grid.
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

fn depth_lit_color(hex: Hex) -> LinearRgba {
    let [_, row] = hex.to_offset_coordinates(OffsetHexMode::Odd, HexOrientation::Pointy);
    let depth = (row as f32 / (MAP_HEIGHT as f32 - 1.0)).clamp(0.0, 1.0);
    LIT_TOP.mix(&LIT_BOTTOM, depth)
}

fn tile_color_for(discovery: &DiscoveryMap, hex: Hex) -> Color {
    let level = discovery.discovered.get(&hex).copied().unwrap_or(0.0);
    HIDDEN.mix(&depth_lit_color(hex), level).into()
}

fn linear_to_srgb_byte(linear: f32) -> u8 {
    let srgb = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).clamp(0.0, 255.0) as u8
}

fn pointy_hex_alpha(dx: f32, dy: f32, r: f32) -> f32 {
    // Pointy-top hex with circumradius r centered at origin. Returns smooth
    // coverage in 0..1 with a 1-pixel anti-alias band at the edge.
    let sqrt3 = 3.0_f32.sqrt();
    let ax = dx.abs();
    let ay = dy.abs();
    let dist_vert = r * sqrt3 * 0.5 - ax;
    let dist_diag = (r * sqrt3 - ax - ay * sqrt3) * 0.5;
    let signed = dist_vert.min(dist_diag);
    (signed + 0.5).clamp(0.0, 1.0)
}

fn build_terrain_atlas() -> Image {
    let cell_w = TILE_PX_W as u32;
    let cell_h = TILE_PX_H as u32;
    let n = TERRAIN_TYPES.len() as u32;
    let total_h = cell_h * n;
    let mut data = vec![0u8; (cell_w * total_h * 4) as usize];

    let center_x = cell_w as f32 * 0.5;
    let center_y = cell_h as f32 * 0.5;
    // Inscribe the hex so it fits the cell on whichever axis is tighter.
    let hex_radius = (cell_h as f32 * 0.5).min(cell_w as f32 / 3.0_f32.sqrt());

    for (idx, &terrain) in TERRAIN_TYPES.iter().enumerate() {
        let linear = terrain_base_color(terrain);
        let r = linear_to_srgb_byte(linear.red);
        let g = linear_to_srgb_byte(linear.green);
        let b = linear_to_srgb_byte(linear.blue);
        let row_offset = idx as u32 * cell_h;

        for py in 0..cell_h {
            for px in 0..cell_w {
                let dx = (px as f32 + 0.5) - center_x;
                let dy = (py as f32 + 0.5) - center_y;
                let alpha = pointy_hex_alpha(dx, dy, hex_radius);
                let global_y = row_offset + py;
                let i = ((global_y * cell_w + px) * 4) as usize;
                data[i] = r;
                data[i + 1] = g;
                data[i + 2] = b;
                data[i + 3] = (alpha * 255.0).clamp(0.0, 255.0) as u8;
            }
        }
    }

    Image::new(
        Extent3d {
            width: cell_w,
            height: total_h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

pub fn spawn_terrain_tilemap(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    grid: Res<GridWorld>,
    layout: Res<HexLayout>,
    discovery: Res<DiscoveryMap>,
    tiles: Query<&Tile>,
) {
    let texture: Handle<Image> = images.add(build_terrain_atlas());

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

    // Decorate each existing simulation entity with a TileBundle. Skip hexes
    // whose offset coordinates fall outside the rectangular tilemap bounds —
    // `TileStorage::set` indexes a flat Vec and would multiply-overflow on
    // wrapped u32 values.
    for (&hex, &entity) in &grid.tiles {
        let Ok(tile) = tiles.get(entity) else {
            continue;
        };
        let Some(tp) = hex_to_tile_pos(hex) else {
            continue;
        };
        if tp.x >= map_size.x || tp.y >= map_size.y {
            continue;
        }
        commands.entity(entity).insert(TileBundle {
            position: tp,
            tilemap_id: TilemapId(tilemap_entity),
            texture_index: TileTextureIndex(terrain_type_index(tile.terrain)),
            color: TileColor(tile_color_for(&discovery, hex)),
            ..Default::default()
        });
        storage.set(&tp, entity);
    }

    // Align tilemap world space with hexx world space. `center_in_world` is
    // tilemap-local; `hex_to_world_pos` is the engine-wide truth. Translate
    // the tilemap so the two agree at Hex::ZERO.
    let zero_tp = hex_to_tile_pos(Hex::ZERO).expect("Hex::ZERO is in-bounds");
    let local = zero_tp.center_in_world(
        &map_size,
        &grid_size,
        &tile_size,
        &map_type,
        &TilemapAnchor::None,
    );
    let world = layout.hex_to_world_pos(Hex::ZERO);
    let origin = (world - local).extend(TERRAIN_Z);

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

// `ParamSet` is the only way to overlap two `&mut TileColor` queries; the
// combined type is wide enough to trip `type_complexity`. Suppress here so
// the workspace lint stays strict for systems that don't need the escape.
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
    // Warn exactly once if a tile entity lacks a TilePos — a stale spawn loop
    // would otherwise emit thousands of warnings per frame.
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
        color.0 = tile_color_for(&discovery, gpos.0);
    }

    // Path 2: discovery sweep, exactly once per sim tick when DiscoveryMap mutates.
    if discovery.is_changed() {
        for (gpos, mut color) in &mut sets.p1() {
            color.0 = tile_color_for(&discovery, gpos.0);
        }
    }
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

    #[test]
    fn pointy_hex_alpha_inside_is_opaque_outside_is_transparent() {
        let r = (TILE_PX_H * 0.5).min(TILE_PX_W / 3.0_f32.sqrt());
        assert!(
            pointy_hex_alpha(0.0, 0.0, r) > 0.99,
            "center should be fully opaque"
        );
        assert_eq!(
            pointy_hex_alpha(TILE_PX_W * 0.5 - 0.5, TILE_PX_H * 0.5 - 0.5, r),
            0.0,
            "cell corner should be fully transparent (outside hex)"
        );
    }

    #[test]
    fn build_terrain_atlas_has_expected_dimensions() {
        let atlas = build_terrain_atlas();
        let size = atlas.texture_descriptor.size;
        assert_eq!(size.width, TILE_PX_W as u32);
        assert_eq!(size.height, TILE_PX_H as u32 * TERRAIN_TYPES.len() as u32);
    }

    #[test]
    fn depth_gradient_top_is_warm_bottom_is_cool() {
        let top = depth_lit_color(Hex::from_offset_coordinates(
            [0, 0],
            OffsetHexMode::Odd,
            HexOrientation::Pointy,
        ));
        let bottom = depth_lit_color(Hex::from_offset_coordinates(
            [0, (MAP_HEIGHT - 1) as i32],
            OffsetHexMode::Odd,
            HexOrientation::Pointy,
        ));
        assert!(
            top.red > top.blue,
            "surface tint should be warm (red>blue), got {top:?}"
        );
        assert!(
            bottom.blue > bottom.red,
            "deep tint should be cool (blue>red), got {bottom:?}"
        );
    }
}

#[cfg(test)]
mod tilemap_tests {
    use super::*;
    use bevy::MinimalPlugins;
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use kingdom_core::{
        GridPos, GridWorld, Hex, HexOrientation, OffsetHexMode, TerrainType, Tile,
        create_hex_layout,
    };
    // NOTE: `super::*` exposes only items actually defined in `terrain_render.rs`;
    // names brought in via `use kingdom_core::{...}` at the top of that file are
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
