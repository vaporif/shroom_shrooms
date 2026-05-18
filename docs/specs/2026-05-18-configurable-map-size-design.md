# Configurable map size

Status: approved
Date: 2026-05-18
Scope: world generation, rendering, CLI, input

## Goal

Make the map dimensions and hive count runtime-configurable from the command
line, and raise the default to a Civ "ultrabig" 220x120. Today the map is a
fixed 80x60, hardcoded as `const`s in two places. The player should be able to
pass `--width`, `--height`, and `--hives` to experiment with map scale without
recompiling.

## Background

The map is 80x60 = 4,800 tiles. `MAP_WIDTH`/`MAP_HEIGHT` are declared twice —
`crates/world/src/terrain_gen.rs` (`i32`) and `crates/render/src/terrain_render.rs`
(`u32`) — and can drift. `terrain_gen` already takes `Res<LaunchConfig>` for the
RNG seed, and `GridWorld` already stores `width`/`height`. The seed flows
CLI -> `Args` -> `LaunchConfig`, a pattern this change reuses for the dimensions.

## Design

### `LaunchConfig` (crates/core/src/config.rs)

Add three fields:

```rust
pub struct LaunchConfig {
    pub seed: u64,
    pub width: i32,
    pub height: i32,
    pub hives: u32,
}
```

New public constants for the defaults: `DEFAULT_MAP_WIDTH = 220`,
`DEFAULT_MAP_HEIGHT = 120`. The `Default` impl uses those, with `hives` set to
the area-scaled count for the default size. Tests that build `LaunchConfig`
directly keep working.

### CLI (bin/src/cli.rs, bin/src/main.rs)

`Args` gains `--width`, `--height`, `--hives`, all `Option`, mirroring `--seed`.
`main.rs` resolves them into `LaunchConfig`:

- `width` / `height`: the flag value, or `DEFAULT_MAP_WIDTH` / `DEFAULT_MAP_HEIGHT`.
- `hives`: the `--hives` value, or an area-scaled default — `width * height /
  TILES_PER_HIVE`, where `TILES_PER_HIVE` keeps the current 4800/6 density (~800).
  At the default size this yields ~33 hives.

### Generator (crates/world/src/terrain_gen.rs)

Delete the local `MAP_WIDTH`/`MAP_HEIGHT` consts. Read `config.width` /
`config.height` for the tile grid, and `config.hives` for the hive placement
loop (replacing `HIVE_COUNT`). Wildlife spawn counts (fungi, plants, bacteria)
scale by `area / BASELINE_AREA`, where `BASELINE_AREA = 4800` — so a 5.5x larger
map gets ~5.5x the wildlife. Fragment count stays `3..=5`: fragments drive the
win condition (`fragments_total` / `mushrooms_required`), so scaling them would
move the goalposts. `grid.width` / `grid.height` are still set from the resolved
dimensions.

### Renderer (crates/render/src/terrain_render.rs)

Delete the local `MAP_WIDTH`/`MAP_HEIGHT` consts. Read `GridWorld.width` /
`GridWorld.height` for the tilemap size, the depth gradient, and the map-bounds
calculations.

### Camera (crates/input/src/camera.rs)

Raise `MAX_ZOOM` from 4.0 so the player can pull back far enough to see a useful
fraction of a 220-wide map. There is no pan clamping in the camera, so panning a
larger map already works without change.

### Tests

`terrain_gen`'s tests assert against the old consts and spawn the whole map.
They pin an explicit small `LaunchConfig` (around 60x40) so each test builds a
small world instead of a 26k-entity one, and the assertions
(`grid.tiles.len()`, the hive count, the player-start position) read the config
values.

## Decisions and trade-offs

### CLI flags on `LaunchConfig` rather than a config file or fixed const

**Pros:** Reuses the exact `--seed` -> `Args` -> `LaunchConfig` path already in
the codebase. No new file format, no parser. Tweaking size needs no recompile.

**Cons:** Settings are not persisted between runs; the player retypes flags.

**Why:** The request is specifically to "play with" the size. CLI flags match
the existing pattern and the iterate-quickly intent. A persisted settings file
is a larger feature with no demand yet.

### Wildlife scales with area, fragments stay fixed

**Pros:** A bigger map keeps a constant wildlife density instead of feeling
empty. Keeping fragments fixed leaves the win condition stable across sizes.

**Cons:** The fragment-hunt covers proportionally less of a big map, so a large
map is, in effect, a longer search.

**Why:** Fragments are the win target. Scaling them would change game length and
balance with map size, which is out of scope here.

### Default 220x120 (~26,400 tiles)

**Pros:** Matches the requested Civ "ultrabig" scale out of the box.

**Cons:** ~5.5x the per-tick simulation work of the current map. The per-tick
systems (`region_tracking`'s connected-components pass, `density_flow`,
`dieback`, `moisture`) are O(tiles).

**Why:** The user asked for this default. Absolute per-tick cost stays small and
Bevy plus the single-chunk tilemap renderer handle 26k entities comfortably; the
implementation verifies the tick is not visibly slow at the default size.

## Execution Strategy

Subagents. The change is one cohesive vertical slice — config field, CLI
resolution, generator, renderer, camera, and tests all move together and share
`LaunchConfig` as the contract. It is a single task with no parallelism, so the
executor dispatches one implementer subagent, then runs spec-compliance and
code-quality review before polish.

## Task Dependency Graph

- Task 1 (Configurable map size) [AFK]: depends on `none` -> single batch

Parallel batches: none.

## Agent Assignments

- Task 1: Configurable map size -> bevy-engineer (Bevy/Rust)
- Polish: post-implementation-polish -> bevy-engineer (uniformly Bevy/Rust diff)

## Out of scope

- A persisted settings file or in-game options menu.
- Scaling the fragment count or the win condition with map size.
- Per-tick simulation performance work beyond confirming the default size is
  playable. If 220x120 proves slow, optimization is a separate project.
- Procedural-generation tuning for large maps (biome distribution, feature
  clustering) beyond the area-scaled wildlife counts.
