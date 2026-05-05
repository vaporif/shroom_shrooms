# bevy_ecs_tilemap Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Replace the per-tile `Mesh2d` + per-tile `TerrainMaterial` renderer in `crates/render/src/terrain_render.rs` with a single `bevy_ecs_tilemap 0.18.1` chunk so per-tick work drops to component writes instead of asset allocation.

**Architecture:** A startup system loads a 7-sprite hex atlas, spawns one tilemap entity, and decorates each existing `GridWorld` tile entity with `TileBundle` (`TilePos`, `TileTextureIndex`, `TileColor`, `TilemapId`). A `PostUpdate` system mutates `TileTextureIndex` on `Changed<Tile>` and `TileColor` on `Changed<Tile> | DiscoveryMap.is_changed()`. `TerrainMaterial`, `TerrainUniforms`, `TerrainSpriteMap`, `terrain_render_system`, `terrain_discovery_update_system`, `build_hex_mesh`, and `bin/assets/shaders/terrain.wgsl` are deleted. `hexx` stays for simulation; only `PlaneMeshBuilder` drops out.

**Tech Stack:** Rust (edition 2024), Bevy 0.18, `bevy_ecs_tilemap 0.18.1`, `hexx 0.24`, `image 0.25` (generator binary only), `cargo nextest`.

## Execution Strategy

**Subagents.** 0 tasks parallelisable, 3 sequential.

Reason: the placeholder atlas (Task 1) is the only work logically independent of integration code, but Task 2 cannot run without it. Task 3 documents the result of Task 2. The whole change lives in one crate with no genuine independence to exploit; team-level coordination would be overhead.

## Task Dependency Graph

- Task 1 [AFK]: depends on `none` → first batch
- Task 2 [AFK]: depends on `Task 1` → second batch
- Task 3 [AFK]: depends on `Task 2` → third batch
- Polish [AFK]: depends on `Task 3` → fourth batch

Each task is independently demoable: Task 1 produces a viewable PNG, Task 2 ships a compiling, running game with the new renderer and passing tests, Task 3 lands accurate docs.

## Agent Assignments

- Task 1: Generate placeholder terrain atlas → rust-engineer (Rust)
- Task 2: bevy_ecs_tilemap integration       → rust-engineer (Rust)
- Task 3: CLAUDE.md cleanup                  → general-purpose (Markdown)
- Polish: post-implementation-polish         → rust-engineer (Rust diff dominates)

---

## File Structure

| File | Status | Responsibility |
|------|--------|----------------|
| `Cargo.toml` (workspace) | Modify | Adds `bevy_ecs_tilemap = "0.18.1"` and `image = "0.25"` to `[workspace.dependencies]`. |
| `bin/Cargo.toml` | Modify | Adds optional `image` dep behind a `gen-atlas` feature so the runtime closure stays unchanged. |
| `bin/src/bin/gen_placeholder_atlas.rs` | Create | One-shot Rust binary that writes the placeholder atlas PNG from the `terrain_base_color` palette. |
| `bin/assets/sprites/terrain/terrain_atlas.png` | Create | 49 × 392 px PNG. 1 column × 7 rows of pointy-top hex sprites. Committed to the repo. |
| `bin/assets/shaders/terrain.wgsl` | Delete | Replaced by atlas sampling done by `bevy_ecs_tilemap`. |
| `crates/render/Cargo.toml` | Modify | Adds `bevy_ecs_tilemap = { workspace = true }` and `fungai_world = { workspace = true }` to `[dependencies]` (the latter so `RenderPlugin` can `.after(fungai_world::terrain_generation)`). |
| `crates/render/src/lib.rs` | Modify | Drops `Material2dPlugin::<TerrainMaterial>` and `TerrainSpriteMap` init, registers `TilemapPlugin`, swaps the two terrain `PostUpdate` systems for `terrain_tile_update_system`, adds `Startup` system `spawn_terrain_tilemap.after(fungai_world::terrain_generation)`, registers `PendingAtlasCheck` resource and `assert_atlas_addresses_all_terrains` Update system. |
| `crates/render/src/terrain_render.rs` | Rewrite | Down from ~230 lines to ~150. Keeps `terrain_base_color` and `terrain_type_index`. Adds `hex_to_tile_pos`, `spawn_terrain_tilemap`, `terrain_tile_update_system`, `assert_atlas_addresses_all_terrains`, and `PendingAtlasCheck` resource. New unit tests: `tilemap_spawns_tile_for_each_hex`, `terrain_tile_update_changes_texture_index`, `discovery_drives_tile_color`, `hex_to_tile_pos_round_trips`, `tilemap_world_pos_aligns_with_hex_layout`, `discovery_applies_to_correct_neighbors`. Removes `terrain_render_spawns_mesh2d_entities`. |
| `CLAUDE.md` | Modify | Crate-name drift fix (`shroom_*` → `fungai_*`); one-line note that terrain rendering uses `bevy_ecs_tilemap`; pointy-top hex clarification. |

## Cross-cutting notes (read before starting any task)

- **Coordinate parity is the sneaky bug.** The simulation uses `OffsetHexMode::Odd` (see `crates/world/src/terrain_gen.rs:28`). Pin `HexCoordSystem::RowOdd` from the start, never `RowEven`. The `hex_to_tile_pos_round_trips` and `discovery_applies_to_correct_neighbors` tests catch parity inversion that single-tile tests miss.
- **World-position alignment matters.** `bevy_ecs_tilemap` places tiles using its own hex math driven by `TilemapTileSize` and `TilemapGridSize`. Network splines, organism sprites, region highlights and priority arrows all consume `hex_layout.hex_to_world_pos` directly — any offset between the two systems shows as visible drift on every other layer. Compute an `origin_offset` at startup and apply it as the tilemap's `Transform.translation`. Do not assume `Transform::IDENTITY` works.
- **Tile entities are the same entity.** Today, simulation tile entities (spawned by `terrain_gen.rs`, tracked in `GridWorld.tiles`) and render mesh entities (spawned by `terrain_render_system`, tracked in `TerrainSpriteMap`) are separate. After this change they are the same entity — render components live alongside `GridPos` and `Tile`. Anyone later writing a `Without<TileTextureIndex>` query against tiles needs to know that.
- **`TileColor` is `bevy_ecs_tilemap`'s newtype**, not `bevy::Color` directly. Convert at the boundary.
- **Z-ordering is now explicit.** Tilemap `Transform.translation.z = -10.0` (terrain backplate). Network/sprite/highlight stay at z=0. Atmosphere keeps positive z.
- **Discovery sweep is per-tick, not per-changed-tile.** Path 2 (`DiscoveryMap.is_changed()`) sweeps all 4800 tile entities and writes `TileColor` once per sim tick. That is intentional. If profiling later shows it mattering, the targeted fix is to emit per-hex deltas from `extract_discovery_map`. Don't pretend Path 2 is "two writes per changed tile".
- **Test plugin set.** Each render test app needs `MinimalPlugins`, `AssetPlugin::default()`, `bevy::image::ImagePlugin::default()`, `bevy_ecs_tilemap::TilemapPlugin`, and the project's `HexLayout` resource via `create_hex_layout()`. Tests must NOT register `extract_discovery_map` — it `.clear()`s `DiscoveryMap` every Update tick, which would erase the manual inserts the discovery tests rely on. No window/render plugins required.
- **Startup ordering is explicit.** `spawn_terrain_tilemap` reads `GridWorld.tiles`, which is populated by `fungai_world::terrain_generation`. Both run in `Startup`, in different plugins; Bevy's scheduler does not honour plugin registration order between unchained `Startup` systems. The plan wires `spawn_terrain_tilemap.after(terrain_generation)` and adds `fungai_world` as a dep of `fungai_render` to make this constraint compile-checked.
- **Dual-query overlap uses `ParamSet`.** `terrain_tile_update_system` mutably touches `TileColor` from two query shapes (Path 1 and Path 2). Bevy rejects two simultaneous `&mut TileColor` queries; `ParamSet<(...)>` is the canonical fix, and it is what the plan ships. Do not reach for `Without<Changed<Tile>>` as a separator — `Without<T>` requires `T: Component` and `Changed<T>` is a `QueryFilter`, so it does not compile.

## Pros / Cons

**Pros**
- One tilemap chunk replaces ~4800 per-tile mesh+material allocations; per-tick work drops to component writes.
- `TerrainMaterial`, `TerrainUniforms`, `TerrainSpriteMap`, the despawn/respawn loop, and the WGSL shader all go away — net deletion ~230 lines + a shader file.
- Per-tile state stays ECS-readable (`TileTextureIndex`, `TileColor` are plain components).
- Opens the door to `bevy_ecs_tilemap`'s animation, autotile, and Tiled-editor features later if we want them.
- Hot-reload of the atlas PNG works out of the box in dev builds.

**Cons**
- Drops the procedural noise/voronoi look. Placeholder atlas ships first; real art swaps in by overwriting the PNG.
- One extra `Hex ↔ TilePos` translation concept at the render boundary (small, isolated, covered by tests).
- A new dep with its own release cadence (mitigated: `bevy_ecs_tilemap` tracks Bevy versions closely).

---

## Task 1: Generate placeholder terrain atlas

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `bin/Cargo.toml`
- Create: `bin/src/bin/gen_placeholder_atlas.rs`
- Create: `bin/assets/sprites/terrain/terrain_atlas.png`

- [x] **Step 1: Add image crate to workspace deps**

In `Cargo.toml`, add to `[workspace.dependencies]` after the existing `leafwing-input-manager` line:

```toml
image = "0.25"
```

- [x] **Step 2: Add gated image dependency to bin**

In `bin/Cargo.toml`, add a `gen-atlas` feature and the optional dep so the runtime closure is unchanged unless someone runs the generator. The block becomes:

```toml
[features]
dev = ["bevy/dynamic_linking", "bevy/hotpatching", "bevy/dev"]
gen-atlas = ["dep:image"]

[dependencies]
bevy = { workspace = true }
clap = { version = "4", features = ["derive"] }
image = { workspace = true, optional = true }
fungai_core = { workspace = true }
fungai_world = { workspace = true }
fungai_growth = { workspace = true }
fungai_regions = { workspace = true }
fungai_render = { workspace = true }
fungai_input = { workspace = true }
fungai_ai = { workspace = true }
fungai_fruiting = { workspace = true }
fungai_ui = { workspace = true }
```

- [x] **Step 3: Create the generator binary**

Write `bin/src/bin/gen_placeholder_atlas.rs`:

```rust
//! Placeholder atlas generator. Run with:
//!   cargo run -p fungai --bin gen_placeholder_atlas --features gen-atlas
//!
//! Writes 1 column x 7 rows of 49x56 pointy-top hex sprites to
//! bin/assets/sprites/terrain/terrain_atlas.png. Each cell is filled with
//! its TerrainType base color plus a tiny dither so it does not read flat;
//! pixels outside the hex polygon are transparent.

#![cfg(feature = "gen-atlas")]

use std::path::PathBuf;

use fungai_core::TerrainType;
use fungai_render::terrain_base_color;
use image::{Rgba, RgbaImage};

const TILE_W: u32 = 49;
const TILE_H: u32 = 56;
const ROWS: u32 = 7;

const TERRAINS: [TerrainType; 7] = [
    TerrainType::Soil,
    TerrainType::Rock,
    TerrainType::Water,
    TerrainType::Root,
    TerrainType::Ruin,
    TerrainType::Toxic,
    TerrainType::Surface,
];

fn srgb_byte(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

fn point_in_pointy_hex(px: f32, py: f32, cx: f32, cy: f32, half_w: f32, half_h: f32) -> bool {
    let dx = (px - cx).abs();
    let dy = (py - cy).abs();
    if dx > half_w || dy > half_h {
        return false;
    }
    dx * (half_h / half_w) + 2.0 * dy <= 2.0 * half_h
}

fn main() {
    let mut img = RgbaImage::new(TILE_W, TILE_H * ROWS);
    let cx = (TILE_W as f32 - 1.0) * 0.5;
    let half_w = (TILE_W as f32 - 1.0) * 0.5;
    let half_h = (TILE_H as f32 - 1.0) * 0.5;

    for (row, terrain) in TERRAINS.iter().copied().enumerate() {
        let base = terrain_base_color(terrain);
        let row_offset = row as u32 * TILE_H;

        for y in 0..TILE_H {
            for x in 0..TILE_W {
                if !point_in_pointy_hex(x as f32, y as f32, cx, half_h, half_w, half_h) {
                    img.put_pixel(x, row_offset + y, Rgba([0, 0, 0, 0]));
                    continue;
                }

                // Cheap deterministic dither: +/- 6% lightness on a 5x5 hash.
                let hash = (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) % 5;
                let dither = match hash {
                    0 => 1.06,
                    1 => 0.94,
                    _ => 1.0,
                };

                img.put_pixel(
                    x,
                    row_offset + y,
                    Rgba([
                        srgb_byte(base.red * dither),
                        srgb_byte(base.green * dither),
                        srgb_byte(base.blue * dither),
                        255,
                    ]),
                );
            }
        }
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("assets/sprites/terrain");
    std::fs::create_dir_all(&path).expect("create assets/sprites/terrain");
    path.push("terrain_atlas.png");
    img.save(&path).expect("write terrain_atlas.png");
    println!("wrote {}", path.display());
}
```

Note: `terrain_base_color` and `TerrainType` are pulled from the project's existing exports. We do **not** introduce a duplicate palette here — the generator is the single consumer of the existing palette function for atlas use.

- [x] **Step 4: Re-export `terrain_base_color` from `fungai_render`**

`crates/render/src/lib.rs` currently does not re-export `terrain_render::terrain_base_color`. The generator imports it from the crate root. Add the re-export. Open `crates/render/src/lib.rs` and after the existing `pub use data_layer::{...};` line add:

```rust
pub use terrain_render::{terrain_base_color, terrain_type_index};
```

(Both will still exist after Task 2; the rewrite keeps them.)

- [x] **Step 5: Build the generator with the feature on**

Run:
```
cargo build -p fungai --bin gen_placeholder_atlas --features gen-atlas
```
Expected: success.

- [x] **Step 6: Run the generator**

Run:
```
cargo run -p fungai --bin gen_placeholder_atlas --features gen-atlas
```
Expected output: `wrote .../bin/assets/sprites/terrain/terrain_atlas.png`.

- [x] **Step 7: Inspect the PNG**

Open `bin/assets/sprites/terrain/terrain_atlas.png` in any image viewer. Verify:
- Image is 49 × 392 px.
- 7 distinct hex sprites stacked vertically.
- Each sprite is pointy-top (point at top), not flat-top.
- Transparent corners around each hex.
- Color order top-to-bottom: dark brown (Soil), grey (Rock), deep blue (Water), dark green (Root), tan (Ruin), olive (Toxic), bright green (Surface).

- [x] **Step 8: Verify the runtime build still excludes image**

Run:
```
cargo build -p fungai
```
Expected: success, and the build does not pull `image` (it is gated behind `gen-atlas`).

To confirm `image` is absent from the runtime closure (it is an optional direct dep of `fungai`, gated by the `gen-atlas` feature; with default features it should not appear at any depth), run:
```
cargo tree -p fungai | rg -c '^[├└]── image\b' || true
```
Expected: prints `0`.

- [x] **Step 9: Commit**

Run:
```
git add Cargo.toml bin/Cargo.toml bin/src/bin/gen_placeholder_atlas.rs bin/assets/sprites/terrain/terrain_atlas.png crates/render/src/lib.rs
git commit -m "render: add placeholder terrain atlas + generator binary"
```

---

## Task 2: bevy_ecs_tilemap integration

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/render/Cargo.toml`
- Modify: `crates/render/src/lib.rs`
- Rewrite: `crates/render/src/terrain_render.rs`
- Delete: `bin/assets/shaders/terrain.wgsl`

### Stage A — wire the dependency

- [x] **Step 1: Add bevy_ecs_tilemap to workspace deps**

In `Cargo.toml`, add to `[workspace.dependencies]` after the new `image` line from Task 1:

```toml
bevy_ecs_tilemap = "0.18.1"
```

Pinning `0.18.1` explicitly: `0.18.0` was a same-day release superseded by `0.18.1`; we avoid the brief window of yanked behaviour.

- [x] **Step 2: Add bevy_ecs_tilemap and fungai_world to the render crate**

In `crates/render/Cargo.toml`, add `bevy_ecs_tilemap` and `fungai_world` to `[dependencies]`. `fungai_world` is needed so `RenderPlugin` can sequence `spawn_terrain_tilemap.after(fungai_world::terrain_generation)`. The block becomes:

```toml
[dependencies]
bevy = { workspace = true }
hexx = { workspace = true }
bevy_ecs_tilemap = { workspace = true }
fungai_core = { workspace = true }
fungai_world = { workspace = true }
```

Note: `fungai_world` already depends on `fungai_core`; pulling it into `fungai_render` does not introduce a cycle (no other render code is referenced from world).

- [x] **Step 3: Build the workspace to verify the dep resolves**

Run: `cargo build -p fungai_render`
Expected: success.

### Stage B — write the failing test (TDD)

- [x] **Step 4: Add the round-trip alignment test first**

Append the following test module to `crates/render/src/terrain_render.rs` (alongside whatever tests are still there). It will not compile yet — `hex_to_tile_pos`, `spawn_terrain_tilemap`, and `terrain_tile_update_system` are intentionally undefined. That is the failing-test signal:

```rust
#[cfg(test)]
mod tilemap_tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use bevy::MinimalPlugins;
    use bevy_ecs_tilemap::prelude::*;
    use fungai_core::{
        create_hex_layout, GridPos, GridWorld, Hex, HexOrientation, OffsetHexMode, TerrainType,
        Tile,
    };
    // NOTE: `super::*` exposes only items actually defined in `terrain_render.rs`;
    // names brought in via `use fungai_core::{...}` at the top of that file are
    // private and do NOT leak through `super::*`. List every type used below
    // explicitly.

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), ImagePlugin::default()));
        app.add_plugins(TilemapPlugin);
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
                Tile { terrain, ..Default::default() },
            ))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(hex, e);
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
            let tp = hex_to_tile_pos(h);
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
        let positions = [Hex::ZERO, Hex::new(1, 0), Hex::new(0, 1), Hex::new(2, -1)];
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
            let tp = hex_to_tile_pos(p);
            if storage.get(&tp).is_some() {
                found += 1;
            }
        }
        assert_eq!(found, positions.len(), "every hex should appear in TileStorage");
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

        let idx = app.world().get::<TileTextureIndex>(entity).expect("tile has index");
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

        let dark = app.world().get::<TileColor>(entity).copied().expect("color exists");

        app.world_mut()
            .resource_mut::<crate::data_layer::DiscoveryMap>()
            .discovered
            .insert(pos, 1.0);
        app.update();

        let lit = app.world().get::<TileColor>(entity).copied().expect("color exists");
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
        let canonical = [
            Hex::ZERO,
            Hex::new(1, 0),
            Hex::new(-1, 0),
            Hex::new(0, 1),
            Hex::new(0, -1),
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
            let tp = hex_to_tile_pos(h);
            let local = tp.center_in_world(
                &tilemap_size,
                &grid_size,
                &tile_size,
                &map_type,
                &TilemapAnchor::None,
            );
            let actual = tilemap_transform.translation.truncate() + local;
            let diff = (actual - expected).length();
            assert!(diff < 1.0, "hex {h:?} drifts by {diff}px (expected={expected:?}, actual={actual:?})");
        }
    }

    #[test]
    fn discovery_applies_to_correct_neighbors() {
        // Even-row vs odd-row neighbour parity. With OffsetHexMode::Odd +
        // HexCoordSystem::RowOdd, lighting hex H must light H, not H's
        // row-shifted lookalike.
        let mut app = test_app();
        let target = Hex::from_offset_coordinates([5, 4], OffsetHexMode::Odd, HexOrientation::Pointy);
        let other = Hex::from_offset_coordinates([5, 5], OffsetHexMode::Odd, HexOrientation::Pointy);

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
            lit_other.0.to_linear().red, baseline_other.0.to_linear().red,
            "neighbour with same offset col but different row must NOT brighten"
        );
    }
}
```

- [x] **Step 5: Confirm the test module fails to compile**

Run: `cargo build -p fungai_render --tests`
Expected: compile error citing missing `hex_to_tile_pos`, `spawn_terrain_tilemap`, `terrain_tile_update_system`. This is the "test fails first" gate. Do not proceed until you see these errors.

### Stage C — implement the new renderer

- [x] **Step 6: Rewrite `crates/render/src/terrain_render.rs`**

Replace the entire file contents (keep the test module from Step 4 at the bottom verbatim, and delete `terrain_render_spawns_mesh2d_entities`, `terrain_material_stores_uniforms`, and the old top-of-file `mod tests` block):

```rust
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use fungai_core::{
    GridPos, GridWorld, Hex, HexLayout, HexOrientation, OffsetHexMode, TerrainType, Tile,
};

use crate::data_layer::DiscoveryMap;

const MAP_WIDTH: u32 = 80;
const MAP_HEIGHT: u32 = 60;

const ATLAS_PATH: &str = "sprites/terrain/terrain_atlas.png";

const TILE_PX_W: f32 = 49.0; // matches the generator's TILE_W
const TILE_PX_H: f32 = 56.0;

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
pub fn hex_to_tile_pos(hex: Hex) -> TilePos {
    let [col, row] = hex.to_offset_coordinates(OffsetHexMode::Odd, HexOrientation::Pointy);
    TilePos {
        x: col as u32,
        y: row as u32,
    }
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
        x: TILE_PX_W,
        y: TILE_PX_H,
    };
    let map_type = TilemapType::Hexagon(HexCoordSystem::RowOdd);

    let tilemap_entity = commands.spawn_empty().id();
    let mut storage = TileStorage::empty(map_size);

    // Decorate each existing simulation entity with a TileBundle.
    for (&hex, &entity) in grid.tiles.iter() {
        let Ok(tile) = tiles.get(entity) else { continue };
        let tp = hex_to_tile_pos(hex);
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
    let zero_tp = hex_to_tile_pos(Hex::ZERO);
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
    if !*warned_untiled {
        if let Some(entity) = untiled.iter().next() {
            warn!(
                "terrain_tile_update_system: entity {entity:?} has Tile but no TilePos -- \
                 spawn_terrain_tilemap likely ran before terrain_generation populated GridWorld"
            );
            *warned_untiled = true;
        }
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
```

Notes for the implementer:
- `ParamSet` is the canonical form for the dual-query overlap on `&mut TileColor`.
  An earlier draft of this plan used `Without<Changed<Tile>>` on the second
  query — that does not compile, because `Without<T>` requires `T: Component`
  and `Changed<T>` is a `QueryFilter`. Do not reintroduce that.
- The `level` lookup is consistent: missing key → 0.0 (hidden). The `extract_discovery_map` system only inserts entries with `discovered > 0.0`, so the absent path is correct.
- Path 1 fires on `Changed<Tile>` only; Path 2 drives all-tile colour updates whenever `DiscoveryMap` mutates. The `discovery_drives_tile_color` and `discovery_applies_to_correct_neighbors` tests rely on Path 2, since they mutate `DiscoveryMap` without changing any `Tile`.
- Keep no `unwrap` paths in production code: the asset load is async and `Tile` query lookup uses `let Ok ... else { continue }`.

- [x] **Step 7: Update `crates/render/src/lib.rs`**

Replace the existing `RenderPlugin::build` body with the version below. The diff:
- Drop `Material2dPlugin::<TerrainMaterial>::default()` and `init_resource::<TerrainSpriteMap>()`.
- Drop the two terrain `PostUpdate` systems.
- Add `bevy_ecs_tilemap::TilemapPlugin`.
- Add `Startup` system `spawn_terrain_tilemap`, ordered explicitly **after** `fungai_world::terrain_generation`. Bevy's `Startup` schedule does not honour plugin registration order — without `.after(...)`, the tilemap may spawn before `GridWorld.tiles` is populated and end up empty. `terrain_generation` is `pub` from `fungai_world` (see `crates/world/src/lib.rs:8`), so the dependency is direct.
- Add `PostUpdate` system `terrain_tile_update_system` first in the chain.

```rust
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy_ecs_tilemap::prelude::TilemapPlugin;
use fungai_core::SimulationSet;
use fungai_world::terrain_generation;

mod assets;
mod atmosphere;
mod data_layer;
mod entity_render;
mod network_render;
mod terrain_render;

pub use data_layer::{
    BranchGraph, DiscoveryMap, PriorityBiasMap, RegionHulls, RivalBranchGraph, TipPositions,
};
pub use network_render::catmull_rom;
pub use terrain_render::{terrain_base_color, terrain_type_index};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .add_plugins(Material2dPlugin::<atmosphere::VignetteMaterial>::default())
            .add_plugins(Material2dPlugin::<network_render::NetworkMaterial>::default())
            .init_resource::<assets::EntitySprites>()
            .init_resource::<terrain_render::PendingAtlasCheck>()
            .init_resource::<BranchGraph>()
            .init_resource::<TipPositions>()
            .init_resource::<RegionHulls>()
            .init_resource::<data_layer::DiscoveryMap>()
            .init_resource::<data_layer::RivalBranchGraph>()
            .init_resource::<data_layer::PriorityBiasMap>()
            .init_resource::<data_layer::SelectedRegionTiles>()
            .add_systems(
                Update,
                (
                    data_layer::extract_branch_graph,
                    data_layer::extract_tip_positions,
                    data_layer::extract_region_hulls,
                    data_layer::extract_discovery_map.after(data_layer::extract_branch_graph),
                    data_layer::extract_rival_branch_graph,
                )
                    .in_set(SimulationSet),
            )
            .add_systems(
                Update,
                (
                    data_layer::extract_priority_bias_map,
                    data_layer::extract_selected_region_tiles,
                    terrain_render::assert_atlas_addresses_all_terrains,
                ),
            )
            .add_systems(
                Startup,
                (
                    assets::load_entity_sprites,
                    atmosphere::spawn_vignette,
                    atmosphere::spawn_particle_pool,
                    terrain_render::spawn_terrain_tilemap.after(terrain_generation),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    terrain_render::terrain_tile_update_system,
                    network_render::network_render_system,
                    entity_render::tip_render_system,
                    (
                        entity_render::despawn_orphaned_organism_sprites,
                        entity_render::spawn_organism_sprites,
                    )
                        .chain(),
                    entity_render::priority_arrow_render_system,
                    entity_render::region_highlight_render_system,
                    atmosphere::update_vignette,
                    atmosphere::update_particles,
                ),
            );
    }
}
```

The `.after(terrain_generation)` constraint is load-bearing: `WorldPlugin::build` (`crates/world/src/lib.rs:14`) registers `terrain_generation` in the same `Startup` schedule and the systems would otherwise race. The plugin registration order in `bin/src/plugins.rs` (which adds `WorldPlugin` before `RenderPlugin`) is **not** sufficient — Bevy schedules unordered `Startup` systems in arbitrary order across plugins.

- [x] **Step 8: Build with the failing tests still in place**

Run: `cargo build -p fungai_render --tests`
Expected: success. The tests added in Step 4 should now compile.

- [x] **Step 9: Run the new tilemap tests**

Run: `cargo nextest run -p fungai_render tilemap`
Expected: all six tests in `tilemap_tests` pass:
- `hex_to_tile_pos_round_trips`
- `tilemap_spawns_tile_for_each_hex`
- `terrain_tile_update_changes_texture_index`
- `discovery_drives_tile_color`
- `tilemap_world_pos_aligns_with_hex_layout`
- `discovery_applies_to_correct_neighbors`

If `tilemap_world_pos_aligns_with_hex_layout` fails, do NOT loosen the `< 1.0` tolerance. The test is the source of truth on parity and spacing — adjust `TILE_PX_W` / `TILE_PX_H` (and the corresponding generator constants in `bin/src/bin/gen_placeholder_atlas.rs`, then re-run the generator) until the math agrees with `hexx`'s pointy-top geometry (`width = scale * sqrt(3)`, `height = scale * 2`, row stride `= scale * 1.5`).

- [x] **Step 10: Run the full render-crate test suite**

Run: `cargo nextest run -p fungai_render`
Expected: all tests green. The pre-existing data-layer tests still pass; the deleted tests (`terrain_render_spawns_mesh2d_entities`, `terrain_material_stores_uniforms`) no longer appear.

- [x] **Step 11: Delete the now-dead shader**

Run: `git rm bin/assets/shaders/terrain.wgsl`

- [x] **Step 12: Confirm no leftover references**

Run:
```
rg 'TerrainMaterial|TerrainUniforms|TerrainSpriteMap|terrain_render_system|terrain_discovery_update_system|build_hex_mesh|terrain\.wgsl' .
```
Expected: zero matches anywhere in the repo (including comments and documentation).

- [x] **Step 13: Workspace build**

Run: `cargo build`
Expected: success.

- [x] **Step 14: Workspace tests**

Run: `just test`
Expected: green across the workspace.

- [x] **Step 15: Lint**

Run: `just lint`
Expected: clean.

### Stage D — manual smoke test

- [x] **Step 16: Run the game and verify the terrain renders**

Run: `just dev`
Verify:
- Terrain hexagons appear across the entire 80×60 grid.
- Colors match the seven terrain types (soil/rock/water/root/ruin/toxic/surface).
- Hex grid is pointy-top oriented (point at top), not flat-top.
- Discovered tiles near the player network appear brighter; undiscovered tiles are dim.
- Network splines, hyphal tip sprites, region highlights, and priority arrows all visually align with the terrain hexes (no per-tile drift).
- Atmosphere vignette and particles are visible in front of the terrain.
- No flicker, no missing tiles, no z-fight artifacts.

Document the manual verification in the PR description with a screenshot if possible.

- [x] **Step 17: Commit**

Run:
```
git add Cargo.toml crates/render/Cargo.toml crates/render/src/lib.rs crates/render/src/terrain_render.rs
git rm bin/assets/shaders/terrain.wgsl
git commit -m "render: switch terrain to bevy_ecs_tilemap"
```

---

## Task 3: CLAUDE.md cleanup

**Files:**
- Modify: `CLAUDE.md`

- [x] **Step 1: Fix crate-name drift**

In `CLAUDE.md`, under the "Workspace Architecture" section, update the listed crate names from `shroom_*` to `fungai_*`. The block currently lists:

```
shroom_core     - Shared types: ...
shroom_world    - Procedural terrain generation, ...
shroom_growth   - Nutrient gradients/transport, ...
shroom_regions  - Specialization, discovery, ...
shroom_ai       - Rival fungi AI, ...
shroom_fruiting - Fruiting body progression, ...
shroom_render   - Two-layer rendering: ...
shroom_input    - Camera (WASD/scroll), ...
shroom_ui       - HUD, ability bar, ...
shroom_shrooms  - Main binary: ...
```

Replace each `shroom_*` with the actual crate name from the workspace (`fungai_core`, `fungai_world`, `fungai_growth`, `fungai_regions`, `fungai_ai`, `fungai_fruiting`, `fungai_render`, `fungai_input`, `fungai_ui`). The main binary lives in `bin/` and is the package `fungai`; update the last line to reflect that.

- [x] **Step 2: Fix the remaining `shroom_*` mentions outside the crate list**

The crate prefix has leaked into two more places in `CLAUDE.md` that the workspace-architecture block alone does not cover:

- Line 22 — `Single test: cargo nextest run -p shroom_core test_name` → change `shroom_core` to `fungai_core`.
- Line 44 — `All domain crates depend on \`shroom_core\`. The main binary in \`shroom_shrooms\` composes all plugins.` → change `shroom_core` to `fungai_core` and `shroom_shrooms` to `fungai` (the binary package). The replacement reads: `All domain crates depend on \`fungai_core\`. The main binary lives in \`bin/\` (package \`fungai\`) and composes all plugins.`

Note: line 84 contains the spec filename `2026-04-19-shroom-shrooms-design.md`. That is a real path on disk (verified at `docs/superpowers/specs/2026-04-19-shroom-shrooms-design.md`); leave it untouched. The hyphenated `shroom-shrooms` slug also doesn't match the verification regex below, which scans only for the underscore-prefixed crate form.

- [x] **Step 3: Update the rendering section**

In the "Rendering" section of `CLAUDE.md`, replace the description of `crates/shroom_shrooms/assets/shaders/` with the post-migration setup. The new wording:

```
Custom WGSL shaders in `bin/assets/shaders/`:
- `network.wgsl`  - Mycelium paths with multi-strand splines, biomass-based thickness
- `vignette.wgsl` - Screen-space atmosphere effect

Terrain uses `bevy_ecs_tilemap` 0.18.1 with a single hex atlas at
`bin/assets/sprites/terrain/terrain_atlas.png`. Per-tile state is expressed
as `TileTextureIndex` and `TileColor` components on the simulation tile
entities; one tilemap chunk replaces ~4800 per-tile mesh+material entities.
The grid is pointy-top hex (`HexCoordSystem::RowOdd`) to match the world
generator's `OffsetHexMode::Odd`.

Sprites in `bin/assets/sprites/` for fauna, fragments, mushrooms, neutral
fungi, plant roots.
```

The wording about Catmull-Rom splines for the network stays as it is; it still applies to network rendering.

- [x] **Step 4: Verify**

Use a regex that matches the crate-prefix form (`shroom_<word>`) but not the hyphenated `shroom-shrooms` filename slug on line 84:
```
rg 'shroom_(core|world|growth|regions|ai|fruiting|render|input|ui|shrooms)\b' CLAUDE.md
```
Expected: zero matches.

Run:
```
rg 'terrain\.wgsl' CLAUDE.md
```
Expected: zero matches.

- [x] **Step 5: Commit**

Run:
```
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for tilemap migration and crate rename"
```

---

## Final verification (after all tasks)

- [x] **Step 1: Full lint**

Run: `just lint`
Expected: clean (no fmt or clippy warnings introduced).

- [x] **Step 2: Full test suite**

Run: `just test`
Expected: green across the workspace.

- [ ] **Step 3: End-to-end smoke test**

Run: `just dev`
Verify (in the running game):
- The full 80×60 grid is visible at startup.
- Pointy-top hex orientation matches the rest of the project's geometry — no rotation drift relative to network splines, organism sprites, region highlights, or priority arrows.
- Discovery shading evolves smoothly as the network grows; freshly-grown frontier hexes brighten over time.
- Terrain renders behind every other layer (splines, sprites, highlights, arrows) and in front of nothing other than the empty background.
- Pause / unpause: discovery sweeps stop while paused (no flicker on `Path 2`).
- Hot-reload: edit `bin/assets/sprites/terrain/terrain_atlas.png` (e.g. re-run the generator with a tweaked palette) and confirm the running dev build picks up the new atlas without restart.

- [x] **Step 4: Confirm no leftover terrain-renderer artifacts**

Run:
```
rg 'PlaneMeshBuilder|TerrainMaterial|TerrainUniforms|TerrainSpriteMap|terrain_render_system|terrain_discovery_update_system|terrain\.wgsl' .
```
Expected: zero matches across the entire repo.

- [x] **Step 5: Confirm dependency closure**

`bevy_ecs_tilemap` is a dep of `fungai_render`, not the `fungai` bin crate, so it appears at depth 1 only when querying the render crate. `image` is a feature-gated direct dep of `fungai`; with default features it should be absent from the runtime closure entirely.

Run:
```
cargo tree -p fungai_render --depth 1 | rg '^[├└]── bevy_ecs_tilemap\b'
```
Expected: exactly one match (the `bevy_ecs_tilemap` line).

Run:
```
cargo tree -p fungai | rg -c '^[├└]── image\b' || true
```
Expected: prints `0` — `image` is gated behind the `gen-atlas` feature and not in the runtime closure.
