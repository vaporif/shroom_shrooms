# Configurable Map Size Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Make map dimensions and hive count runtime-configurable via CLI flags, defaulting to a 220x120 map.

**Architecture:** `LaunchConfig` carries `width`/`height`/`hives`, fed from clap `Args`. The generator reads `LaunchConfig`; the renderer reads `GridWorld.width/height`. The two duplicated `MAP_WIDTH`/`MAP_HEIGHT` consts are removed.

**Tech Stack:** Rust (edition 2024), Bevy 0.18, clap, `bevy_ecs_tilemap`, `cargo nextest`.

## Starting state (read before doing anything)

This feature is **partly implemented in the working tree, uncommitted, and the build is broken**. Before this plan was written, someone converted the config/CLI layer and started on the generator. Confirm this state with `git status` and `cargo build -p kingdom` before Task 1.

Already done and correct (uncommitted — do NOT redo, just commit it as part of Task 1):
- `crates/core/src/config.rs` — `LaunchConfig` has `seed`/`width`/`height`/`hives`; `DEFAULT_MAP_WIDTH = 220`, `DEFAULT_MAP_HEIGHT = 120`, `TILES_PER_HIVE = 800`, `default_hive_count()`, the `Default` impl, and four tests. Complete.
- `bin/src/cli.rs` — `--width`, `--height`, `--hives` flags plus six parse tests. Complete.
- `bin/src/main.rs` — resolves `width`/`height`/`hives` from `Args` and builds `LaunchConfig`. Complete.

Half done, **does not compile** (Task 1 finishes it):
- `crates/world/src/terrain_gen.rs` — `BASELINE_AREA = 4800` added, the `MAP_WIDTH`/`MAP_HEIGHT` consts deleted, `terrain_generation`/`pick_terrain`/`build_tile_data` thread `width`/`height`, and the hive loop uses `config.hives`. But `build_soil_pool`, `place_features`, and `build_tile_buffer` still have their old signatures and bodies that reference the deleted consts (the call sites already pass `width`/`height`, so the arities mismatch). The wildlife counts are not yet area-scaled. The test module still constructs `LaunchConfig { seed: 12345 }` (one field) and references `MAP_WIDTH`/`MAP_HEIGHT`/`HIVE_COUNT`. `cargo build -p kingdom` currently fails with ~12 errors here.

Not started:
- `crates/render/src/terrain_render.rs` — still has its own `const MAP_WIDTH: u32 = 80` / `MAP_HEIGHT: u32 = 60` (marked `// TODO: to change`).
- `crates/input/src/camera.rs` — `MAX_ZOOM` still `4.0`.

## Execution Strategy

Subagents. The change is one cohesive vertical slice split into three small sequential tasks — finish the generator, convert the renderer, adjust the camera — that share `LaunchConfig`/`GridWorld` as the contract. The executor dispatches one implementer subagent per task, each after the previous task's review passes, then runs polish.

## Task Dependency Graph

- Task 1 (Finish the generator conversion) [AFK]: depends on `none` → batch 1
- Task 2 (Convert the renderer to runtime dimensions) [AFK]: depends on `Task 1` → batch 2
- Task 3 (Camera zoom-out range and full verification) [AFK]: depends on `Task 2` → batch 3

Parallel batches: none. Tasks 2 and 3 touch separate crates from Task 1, but the workspace test suite cannot pass until Task 1 makes `kingdom_world` compile, so they run as a sequential chain.

## Agent Assignments

- Task 1: Finish the generator conversion → bevy-engineer (Bevy/Rust)
- Task 2: Convert the renderer to runtime dimensions → bevy-engineer (Bevy/Rust)
- Task 3: Camera zoom-out range and full verification → bevy-engineer (Bevy/Rust)
- Polish: post-implementation-polish → bevy-engineer (uniformly Bevy/Rust diff)

---

## File Structure

| File | Change |
|---|---|
| `crates/core/src/config.rs` | Already done — `LaunchConfig` fields, defaults, `default_hive_count`. Committed in Task 1. |
| `bin/src/cli.rs`, `bin/src/main.rs` | Already done — CLI flags and resolution. Committed in Task 1. |
| `crates/world/src/terrain_gen.rs` | Task 1 — finish the helper signatures, area-scale wildlife, fix the test module. |
| `crates/render/src/terrain_render.rs` | Task 2 — drop the local consts, read `GridWorld` dimensions. |
| `crates/input/src/camera.rs` | Task 3 — raise `MAX_ZOOM`. |

---

## Task 1: Finish the generator conversion

Make `kingdom_world` compile and its tests pass. Finish the three helper functions left half-converted, area-scale the wildlife counts, and repair the test module. Commit the already-complete config/CLI/main work alongside.

**Files:**
- Modify: `crates/world/src/terrain_gen.rs`

### Background

`terrain_generation` already binds `let (width, height) = (config.width, config.height);` and passes both into `build_tile_data`, `build_soil_pool`, `place_features`, and `build_tile_buffer`. The last three functions were not updated to accept or use them — that is the compile break. The wildlife loops in `place_features` still use fixed counts; they should scale with map area. Fragments stay fixed: they set `game_state.fragments_total` / `mushrooms_required`, the win condition.

- [x] **Step 1: Confirm the broken state**

Run: `cargo build -p kingdom_world`
Expected: FAIL — `cannot find value MAP_WIDTH/MAP_HEIGHT in this scope`, plus arity mismatches for `build_soil_pool` / `place_features` / `build_tile_buffer`.

- [x] **Step 2: Add the area-scaling helper**

In `crates/world/src/terrain_gen.rs`, add this free function (place it after `offset_to_hex`):

```rust
/// Scale a base spawn count by map area relative to the 80x60 baseline,
/// clamped to at least 1 so a small map still gets some wildlife.
fn area_scaled(base: u32, width: i32, height: i32) -> u32 {
    let scale = (width * height) as f32 / BASELINE_AREA as f32;
    ((base as f32 * scale).round() as u32).max(1)
}
```

- [x] **Step 3: Fix `build_soil_pool`**

Change the signature and body so it takes `width`/`height` instead of the deleted consts:

```rust
/// Soil hexes available for feature placement, excluding map borders and the surface row.
fn build_soil_pool(
    tile_data: &HashMap<Hex, TileBase>,
    rng: &mut StdRng,
    width: i32,
    height: i32,
) -> Vec<Hex> {
    let mut pool = Vec::new();
    for y in 1..height - 2 {
        for x in 1..width - 1 {
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
```

- [x] **Step 4: Fix `place_features` — signature and the plant loop coordinates**

Change the signature to accept `width`/`height`:

```rust
fn place_features(
    rng: &mut StdRng,
    tile_data: &mut HashMap<Hex, TileBase>,
    soil_pool: &mut Vec<Hex>,
    game_state: &mut GameState,
    width: i32,
    height: i32,
) -> Placements {
```

Inside the plant loop, the two `MAP_WIDTH`/`MAP_HEIGHT` references become `width`/`height`:

```rust
        let x = rng.random_range(0..width);
        let y = rng.random_range(height / 2..height - 1);
```

- [x] **Step 5: Area-scale the wildlife counts in `place_features`**

The fungi, plant, and bacteria loops get area-scaled counts. The fragment loop and the `UniqueDecomposable` loop stay fixed — fragments are the win condition, and decomposables are out of scope for area-scaling per the design spec. Roll the base range first (one RNG draw, so the seed stays deterministic), then scale:

Fungi loop — was `for i in 0..rng.random_range(2u32..=4)`:

```rust
    let fungi_count = area_scaled(rng.random_range(2u32..=4), width, height);
    for i in 0..fungi_count {
```

Plant loop — was `for i in 0..rng.random_range(3u32..=6)`:

```rust
    let plant_count = area_scaled(rng.random_range(3u32..=6), width, height);
    for i in 0..plant_count {
```

Bacteria loop — was `for _ in 0..rng.random_range(1u32..=2)`:

```rust
    let bacteria_count = area_scaled(rng.random_range(1u32..=2), width, height);
    for _ in 0..bacteria_count {
```

Leave the fragment loop (`rng.random_range(3u32..=5)` feeding `fragments_total`) and the `UniqueDecomposable` loop (`rng.random_range(3u32..=5)`) exactly as they are.

- [x] **Step 6: Fix `build_tile_buffer`**

Change the signature and the loop bounds:

```rust
fn build_tile_buffer(
    tile_data: &HashMap<Hex, TileBase>,
    placements: &mut Placements,
    player_rid: RegionId,
    player_hexes: &HashSet<Hex>,
    width: i32,
    height: i32,
) -> Vec<(Hex, Tile)> {
    let mut buf = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
```

The rest of the body is unchanged.

- [x] **Step 7: Build the crate (non-test)**

Run: `cargo build -p kingdom_world`
Expected: PASS — the library compiles. (Tests may still fail to compile; that is Step 8.)

- [x] **Step 8: Repair the `terrain_gen` test module**

The test module at the bottom of the file references the deleted consts and builds a one-field `LaunchConfig`. Replace its `test_app` helper and add explicit test dimensions. Pin a small map so each test builds a ~2,400-tile world instead of 26,400.

Add test consts at the top of the `tests` module (after `use` lines):

```rust
    const TEST_WIDTH: i32 = 60;
    const TEST_HEIGHT: i32 = 40;
    const TEST_HIVES: u32 = 3;
```

Replace the `LaunchConfig` line in `test_app`:

```rust
        app.insert_resource(LaunchConfig {
            seed: 12345,
            width: TEST_WIDTH,
            height: TEST_HEIGHT,
            hives: TEST_HIVES,
        });
```

Update each test that referenced the old consts:
- `generates_grid_with_correct_dimensions`: assert against `TEST_WIDTH` / `TEST_HEIGHT`, and `grid.tiles.len()` against `(TEST_WIDTH * TEST_HEIGHT) as usize`.
- `top_row_is_surface_terrain`: `for x in 0..TEST_WIDTH`, `offset_to_hex(x, TEST_HEIGHT - 1)`.
- `moisture_higher_near_surface`: `offset_to_hex(0, TEST_HEIGHT - 2)`.
- `places_hives_clear_of_player_start`: `offset_to_hex(TEST_WIDTH / 2, TEST_HEIGHT / 2)` for `player_start`, and `assert!(hive_count > 0 && hive_count <= TEST_HIVES as i32)`.

The other tests (`places_fragments`, `start_region_starts_with_full_sugars`, `fragment_tiles_preserve_rng_nutrient_and_moisture`) do not reference the consts and need no change.

- [x] **Step 9: Run the `terrain_gen` tests**

Run: `cargo nextest run -p kingdom_world`
Expected: PASS — every `kingdom_world` test green, including the `terrain_gen` and `region_tracking` tests.

- [x] **Step 10: Commit**

The config/CLI/main changes are already staged-worthy and complete; commit them together with this task's generator fixes.

Run: `git add -A && git commit -m "world: configurable map dimensions and area-scaled wildlife"`

---

## Task 2: Convert the renderer to runtime dimensions

`terrain_render.rs` still hardcodes an 80x60 map. Make it read the real dimensions from `GridWorld`, which the generator populates.

**Files:**
- Modify: `crates/render/src/terrain_render.rs`

### Background

`spawn_terrain_tilemap` already takes `grid: Res<GridWorld>` — `GridWorld` has `width: i32` / `height: i32`. Two functions use the consts beyond that system: `depth_lit_color(hex)` (the depth gradient needs the map height) and its caller `tile_color_for(discovery, hex)`. Both gain a `height` parameter. `terrain_tile_update_system` calls `tile_color_for` and so needs `Res<GridWorld>` too.

- [ ] **Step 1: Delete the local consts**

Remove these two lines (and the `// TODO: to change` comment above them) from `crates/render/src/terrain_render.rs`:

```rust
// TODO: to change
const MAP_WIDTH: u32 = 80;
const MAP_HEIGHT: u32 = 60;
```

- [ ] **Step 2: Thread `height` through `depth_lit_color` and `tile_color_for`**

```rust
fn depth_lit_color(hex: Hex, map_height: u32) -> LinearRgba {
    let [_, row] = hex.to_offset_coordinates(OffsetHexMode::Odd, HexOrientation::Pointy);
    let denom = (map_height.max(1) as f32 - 1.0).max(1.0);
    let depth = (row as f32 / denom).clamp(0.0, 1.0);
    LIT_TOP.mix(&LIT_BOTTOM, depth)
}

fn tile_color_for(discovery: &DiscoveryMap, hex: Hex, map_height: u32) -> Color {
    let level = discovery.discovered.get(&hex).copied().unwrap_or(0.0);
    HIDDEN.mix(&depth_lit_color(hex, map_height), level).into()
}
```

The `denom` guard keeps a 0- or 1-row map (or an empty `GridWorld` before generation) from dividing by zero or a negative.

- [ ] **Step 3: Read dimensions from `GridWorld` in `spawn_terrain_tilemap`**

In `spawn_terrain_tilemap`, replace the `TilemapSize` construction. Add, near the top of the function:

```rust
    let map_w = grid.width.max(0) as u32;
    let map_h = grid.height.max(0) as u32;
```

Change `map_size`:

```rust
    let map_size = TilemapSize { x: map_w, y: map_h };
```

Update the `tile_color_for` call in the tile loop to pass `map_h`:

```rust
            color: TileColor(tile_color_for(&discovery, hex, map_h)),
```

- [ ] **Step 4: Give `terrain_tile_update_system` the map height**

Add `grid: Res<GridWorld>` to the system's parameters. Near the top of the body:

```rust
    let map_h = grid.height.max(0) as u32;
```

Update both `tile_color_for` calls (the per-changed-tile path and the discovery sweep) to pass `map_h`:

```rust
        color.0 = tile_color_for(&discovery, gpos.0, map_h);
```

- [ ] **Step 5: Fix the `depth_gradient_top_is_warm_bottom_is_cool` test**

In the `tests` module, that test calls `depth_lit_color(hex)` and uses `MAP_HEIGHT`. Give it an explicit height. Add a test const at the top of the `tests` module:

```rust
    const TEST_MAP_HEIGHT: u32 = 60;
```

Then update the test's two `depth_lit_color` calls to pass `TEST_MAP_HEIGHT`, and replace `(MAP_HEIGHT - 1) as i32` with `(TEST_MAP_HEIGHT - 1) as i32`.

The `tilemap_tests` module uses offset-coordinate literals (`[79, 0]`, `[0, 59]`, etc.) rather than the consts, so it compiles unchanged.

- [ ] **Step 6: Build and test the render crate**

Run: `cargo nextest run -p kingdom_render`
Expected: PASS — all render tests green.

- [ ] **Step 7: Commit**

Run: `git add -A && git commit -m "render: read map dimensions from GridWorld"`

---

## Task 3: Camera zoom-out range and full verification

Raise the camera zoom-out limit so a 220-wide map is viewable, then verify the whole feature end to end.

**Files:**
- Modify: `crates/input/src/camera.rs`

- [ ] **Step 1: Raise `MAX_ZOOM`**

In `crates/input/src/camera.rs`, change:

```rust
const MAX_ZOOM: f32 = 4.0;
```

to:

```rust
const MAX_ZOOM: f32 = 12.0;
```

- [ ] **Step 2: Update the `zoom_range_matches_spec` test**

That test asserts `MAX_ZOOM == 4.0`. Change the assertion to `assert_eq!(MAX_ZOOM, 12.0);`. Leave `MIN_ZOOM` and `zoom_factor_is_multiplicative_uniform` alone — the multiplicative test derives its range from `MAX_ZOOM`/`MIN_ZOOM` and still holds.

- [ ] **Step 3: Test the input crate**

Run: `cargo nextest run -p kingdom_input`
Expected: PASS.

- [ ] **Step 4: Full workspace lint and test**

Run: `just lint && just test`
Expected: PASS — `cargo fmt --check`, clippy, and `bevy_lint` clean; every test green. If `cargo fmt --check` fails because the editor's formatter hook ordered imports differently, run `just fmt` and re-run.

- [ ] **Step 5: Verify the default resolves to 220x120**

Run: `cargo nextest run -p kingdom_core config`
Expected: PASS — `launch_config_default_dimensions_are_220x120` and `launch_config_default_hive_count_is_area_scaled` confirm the default `LaunchConfig` is 220x120 with 33 hives.

- [ ] **Step 6: Smoke-build the binary**

Run: `cargo build -p kingdom`
Expected: PASS — the game builds with the new defaults. The map now generates at 220x120 (26,400 tiles) unless `--width` / `--height` override it; `--hives` overrides the area-scaled hive count.

- [ ] **Step 7: Commit**

Run: `git add -A && git commit -m "input: widen camera zoom-out range for large maps"`

---

## Verification

The feature is complete when:

- `just lint` is clean and `just test` is fully green.
- `cargo build -p kingdom` succeeds.
- `cargo run -p kingdom` generates a 220x120 map; `cargo run -p kingdom -- --width 128 --height 80 --hives 12` generates a 128x80 map with 12 hives.
- The `kingdom_core` config tests confirm the 220x120 / 33-hive default.

A manual check (`just run`) at the default size is worth doing — confirm the map renders, the camera zooms out far enough to see a useful slice, and a tick is not visibly slow. Per-tick simulation cost at 26,400 tiles is ~5.5x the old map; if a tick is visibly slow, that is a separate optimization, out of scope here.
