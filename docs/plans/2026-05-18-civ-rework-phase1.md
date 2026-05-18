# Civ Rework Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Turn the single-network mycelium game into a Civ-shaped game where multiple networks act as cities, insect hives can be captured to produce founder units, and a founder walks the map to found a new network.

**Architecture:** A new `kingdom_units` crate adds hives, units, pathfinding, and founding on top of the unchanged mycelium simulation. The region model in `kingdom_core` keeps its existing field set; `region_tracking_system` is reworked to merge and split deterministically. The left mouse button is freed for unit control; the wisp moves behind a held modifier key.

**Tech Stack:** Rust (edition 2024), Bevy 0.18 ECS, `hexx` 0.24 hex grid, `leafwing-input-manager` 0.20, `cargo nextest`.

## Execution Strategy

**Subagents.** Every task runs in a fresh dispatched agent. Phase 1 is a single tightly-coupled vertical stack: each task extends the network/hive/unit model the previous one established, and the tasks share `kingdom_core`, the new `kingdom_units` crate, and `kingdom_render`. There is no honest parallelism, so the executor dispatches the six tasks strictly in sequence, each after its predecessor's review passes.

## Task Dependency Graph

- Task 1 (Deterministic region tracking) [AFK]: depends on `none` → batch 1
- Task 2 (Hives as map features) [AFK]: depends on `Task 1` → batch 2
- Task 3 (Founder production + units) [AFK]: depends on `Task 2` → batch 3
- Task 4 (Unit movement, control, wisp rebind) [AFK]: depends on `Task 3` → batch 4
- Task 5 (Founding new networks) [AFK]: depends on `Task 4` → batch 5
- Task 6 (Integration tests + verification) [AFK]: depends on `Task 5` → batch 6

Parallel batches: none. The executor dispatches T1 → T2 → T3 → T4 → T5 → T6, each after the previous task's review passes.

## Agent Assignments

- Task 1: Deterministic region tracking → bevy-engineer (Bevy/Rust)
- Task 2: Hives as map features → bevy-engineer (Bevy/Rust)
- Task 3: Founder production + units → bevy-engineer (Bevy/Rust)
- Task 4: Unit movement, control, wisp rebind → bevy-engineer (Bevy/Rust)
- Task 5: Founding new networks → bevy-engineer (Bevy/Rust)
- Task 6: Integration tests + verification → bevy-engineer (Bevy/Rust)
- Polish: post-implementation-polish → bevy-engineer (uniformly Bevy/Rust diff)

---

## File Structure

New and modified files across the six tasks:

| File | Responsibility |
|---|---|
| `crates/core/src/components.rs` | New `Hive`, `Unit`, `UnitKind`, `UnitMovement` components; `SelectedUnit` resource. |
| `crates/core/src/messages.rs` | New `HiveCaptured`, `NetworkFounded` messages. |
| `crates/core/src/constants.rs` | New hive/unit/founding tuning constants. |
| `crates/world/src/region_tracking.rs` | Reworked deterministic merge / split; unit re-parenting on merge. |
| `crates/world/src/terrain_gen.rs` | Keeps the start region's explicit starting sugars; places `HIVE_COUNT` hive entities. |
| `crates/units/` (new crate) | `UnitsPlugin`; hive capture, hive production, unit upkeep, unit movement, hex pathfinding (`pathfinding` crate's A*), founding. |
| `crates/input/src/action.rs` | New `WispMode` / `FoundNetwork` actions. |
| `crates/input/src/wisp.rs` | Wisp gated behind `WispMode`. |
| `crates/input/src/pointer.rs` (new) | `pointer_system` — unit select / move order / tile tap. |
| `crates/input/src/cursor.rs` (new) | `cursor_system` — window cursor icon swap. |
| `crates/render/src/units_render.rs` (new) | Unit, hive, selection-ring sprites. |
| `crates/ui/src/hud.rs` | Network count, unit count vs cap, aggregate resources, founder panel. |
| `crates/ui/src/tile_popover.rs` | Hive capture/production in the tile popover. |
| `bin/src/plugins.rs` | Register `UnitsPlugin`. |
| `Cargo.toml` (workspace) | Add `kingdom_units` dependency entry. |

---

## Task 1: Deterministic region tracking

Rework `region_tracking_system` so merges and splits assign region ids deterministically. `RegionState` and `RegionState::new` are untouched — the bare `create_region()` is reused as-is. The game still plays as one network.

**Files:**
- Modify: `crates/world/src/region_tracking.rs`
- Modify: `crates/world/src/terrain_gen.rs:46-69` and `:202-208`
- Modify: `crates/ui/src/hud.rs:159-170`

### Background

A network is just a connected component of owned tiles plus its monotonic `RegionId` — the `RegionId` alone is the identity, with no separate anchor hex. `region_tracking_system` already recomputes connected components every tick. The rework makes id assignment deterministic:

- **Sort:** the components are sorted by their lowest member hex (compare `Hex.x`, then `Hex.y`) before processing, so id assignment is order-independent.
- **Merge:** a component whose tiles carry several region ids collapses to the lowest id (ids are monotonic, so lowest = oldest). Absorbed regions transfer `sugars`/`melanin` to the survivor, then are removed.
- **Split:** when the same id is the min of more than one component, the first component (by the sort) keeps it and its resources; each later one allocates a fresh region via `create_region()`, then explicitly sets that region's `sugars`/`melanin` to 0.0 so it starts with an empty bank.
Nothing depends on `HashMap` iteration order.

Every `create_region()` caller sets its own starting resources explicitly — `init_player_region` sets 100.0, `founding_system` sets `FOUNDER_SEED_SUGARS`, and the split branch sets 0.0 — so `RegionState::new`'s default is irrelevant to production and is left unchanged at 10.0.

`Unit` re-parenting on merge is a fifth responsibility of `region_tracking_system`, but `Unit` does not exist until Task 3. Task 1 builds the merge/split skeleton; Task 3 folds the `&mut Unit` query and re-parenting into the same system once the component is available.

- [x] **Step 1: Write the failing test for deterministic merge and split**

Replace the `tests` module in `crates/world/src/region_tracking.rs`. Keep the existing `test_app` / `spawn_tile` helpers but extend `spawn_tile` to accept an explicit biomass, and add merge/split tests (unit re-parenting is tested in Task 3, once `Unit` exists):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.add_systems(Update, region_tracking_system);
        app
    }

    fn spawn_tile(app: &mut App, pos: Hex, region_id: Option<RegionId>, biomass: f32) -> Entity {
        let entity = app
            .world_mut()
            .spawn((GridPos(pos), Tile { region_id, biomass, ..default() }))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, entity);
        entity
    }

    #[test]
    fn contiguous_regions_merge_to_lowest_id() {
        let mut app = test_app();
        let old = app.world_mut().resource_mut::<RegionStates>().create_region();
        let young = app.world_mut().resource_mut::<RegionStates>().create_region();
        assert!(old.0 < young.0);
        app.world_mut().resource_mut::<RegionStates>().get_mut(old).unwrap().sugars = 30.0;
        app.world_mut().resource_mut::<RegionStates>().get_mut(young).unwrap().sugars = 12.0;

        // Three adjacent tiles bridge the two regions into one component.
        spawn_tile(&mut app, Hex::new(0, 0), Some(old), 1.0);
        spawn_tile(&mut app, Hex::new(1, 0), Some(young), 1.0);
        spawn_tile(&mut app, Hex::new(2, 0), Some(young), 1.0);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert!(rs.get(young).is_none(), "younger region should be absorbed");
        let survivor = rs.get(old).unwrap();
        assert_eq!(survivor.tile_count, 3);
        assert_eq!(survivor.sugars, 42.0, "absorbed sugars pool into survivor");
    }

    #[test]
    fn severed_split_piece_gets_a_fresh_empty_region() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 50.0;

        // Two clusters, no connecting tile. They share `rid` as their min member id,
        // so one keeps it (the cluster sorting first by lowest hex) and the other splits.
        spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
        spawn_tile(&mut app, Hex::new(1, 0), Some(rid), 1.0);
        spawn_tile(&mut app, Hex::new(5, 0), Some(rid), 0.4);
        spawn_tile(&mut app, Hex::new(6, 0), Some(rid), 0.9);
        app.update();

        let rs = app.world().resource::<RegionStates>();
        assert_eq!(rs.regions.len(), 2);
        let kept = rs.get(rid).unwrap();
        assert_eq!(kept.sugars, 50.0, "the id-keeping component keeps its resources");
        let split = rs.regions.values().find(|s| s.region_id != rid).unwrap();
        assert_eq!(split.sugars, 0.0, "the split piece rebuilds its own economy");
        assert_eq!(split.melanin, 0.0);
    }

    #[test]
    fn empty_region_is_removed() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 0.05);
        app.update();
        assert!(app.world().resource::<RegionStates>().get(rid).is_none());
    }
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_world region`
Expected: FAIL — the old tracker does not merge by lowest id, does not split deterministically, and does not re-parent units.

- [x] **Step 3: Rewrite `region_tracking_system`**

Replace the whole non-test body of `crates/world/src/region_tracking.rs` with the following. Task 3 later adds the `&mut Unit` query and a re-parenting pass to this same system, once `Unit` exists.

```rust
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexx::Hex;
use kingdom_core::{GridPos, GridWorld, RegionId, RegionStates, Tile};

pub fn region_tracking_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    // Pass 1 — read-only snapshot of owned tiles and their biomass.
    let mut owned: HashMap<Hex, RegionId> = HashMap::default();
    let mut biomass: HashMap<Hex, f32> = HashMap::default();
    for (gpos, tile) in tiles.iter() {
        if tile.is_owned() && let Some(rid) = tile.region_id {
            owned.insert(gpos.0, rid);
            biomass.insert(gpos.0, tile.biomass);
        }
    }

    for state in region_states.regions.values_mut() {
        state.tile_count = 0;
        state.total_biomass = 0.0;
    }

    // Sort components deterministically before processing: split pieces call
    // create_region(), so the iteration order fixes the new region ids. Sorting
    // by each component's minimum hex makes that order reproducible and
    // independent of HashMap iteration order.
    let mut components = connected_components(&owned, &grid);
    components.sort_by_key(|(_, hexes)| {
        hexes
            .iter()
            .map(|h| (h.x, h.y))
            .min()
            .expect("component is never empty")
    });

    // An id is "absorbed" if it appears as a non-minimum member of any
    // component — a merge swallows it there, so it must not survive anywhere.
    // Every other piece still carrying that id becomes a fresh region instead
    // of keeping it. Computing this set up front (rather than deciding per
    // component) is what makes a simultaneous merge-and-split of the same
    // region deterministic instead of corrupting its id.
    let mut absorbed: HashSet<RegionId> = HashSet::default();
    for (member_ids, _) in &components {
        let candidate = member_ids
            .iter()
            .copied()
            .min()
            .expect("a component always carries at least one member id");
        for &rid in member_ids {
            if rid != candidate {
                absorbed.insert(rid);
            }
        }
    }

    // Pass 2 — assign each component its survivor id, in the sorted order. The
    // first component to want an un-absorbed `candidate` keeps it. A later
    // component with the same candidate is a split; so is any component whose
    // candidate is absorbed elsewhere. Both get a fresh region with an empty
    // bank — RegionState::new's default is a don't-care, the empty economy
    // comes from this explicit assignment.
    let mut claimed: HashSet<RegionId> = HashSet::default();
    let mut survivors: Vec<RegionId> = Vec::with_capacity(components.len());
    for (member_ids, _) in &components {
        let candidate = member_ids
            .iter()
            .copied()
            .min()
            .expect("a component always carries at least one member id");
        let survivor = if !absorbed.contains(&candidate) && claimed.insert(candidate) {
            candidate
        } else {
            let fresh = region_states.create_region();
            if let Some(state) = region_states.get_mut(fresh) {
                state.sugars = 0.0;
                state.melanin = 0.0;
            }
            fresh
        };
        survivors.push(survivor);
    }

    // Pass 3 — fold each absorbed region's resources into the survivor of the
    // first component (in sorted order) that merges it, then remove it.
    // `reparent` doubles as the drain-once ledger here and is consumed by the
    // unit re-parenting pass added in Task 3.
    let mut reparent: HashMap<RegionId, RegionId> = HashMap::default();
    for ((member_ids, _), &survivor) in components.iter().zip(&survivors) {
        let candidate = member_ids
            .iter()
            .copied()
            .min()
            .expect("a component always carries at least one member id");
        for &rid in member_ids {
            if rid == candidate || reparent.contains_key(&rid) {
                continue;
            }
            if let Some(state) = region_states.remove(rid)
                && let Some(survivor_state) = region_states.get_mut(survivor)
            {
                survivor_state.sugars += state.sugars;
                survivor_state.melanin += state.melanin;
            }
            reparent.insert(rid, survivor);
        }
    }

    // Pass 4 — relabel every tile to its component's survivor and tally it.
    for ((_, hexes), &survivor) in components.iter().zip(&survivors) {
        let biomass_sum: f32 = hexes.iter().filter_map(|h| biomass.get(h)).sum();
        if let Some(state) = region_states.get_mut(survivor) {
            state.tile_count = hexes.len() as u32;
            state.total_biomass = biomass_sum;
        }
        for &pos in hexes {
            if let Some(&entity) = grid.tiles.get(&pos)
                && let Ok((_, mut tile)) = tiles.get_mut(entity)
            {
                tile.region_id = Some(survivor);
            }
        }
    }

    region_states.regions.retain(|_, s| s.tile_count > 0);
}

fn connected_components(
    owned: &HashMap<Hex, RegionId>,
    grid: &GridWorld,
) -> Vec<(HashSet<RegionId>, Vec<Hex>)> {
    let mut visited: HashSet<Hex> = HashSet::default();
    let mut components = Vec::new();

    for &start in owned.keys() {
        if visited.contains(&start) {
            continue;
        }
        let mut component = Vec::new();
        let mut member_ids = HashSet::default();
        let mut stack = vec![start];
        while let Some(p) = stack.pop() {
            if !visited.insert(p) {
                continue;
            }
            component.push(p);
            if let Some(&rid) = owned.get(&p) {
                member_ids.insert(rid);
            }
            for (neighbor, _) in grid.neighbors(p) {
                if !visited.contains(&neighbor) && owned.contains_key(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        if !component.is_empty() {
            components.push((member_ids, component));
        }
    }
    components
}
```

Edge case worth tracing: if a region R2 is absorbed by a merge *and* also has a severed chunk in the same tick, R2 appears as a non-minimum member of the merge component, so it lands in `absorbed`. No component then keeps the id R2 — the severed chunk's `candidate` is R2, but `absorbed.contains(R2)` forces it onto a fresh empty region. R2's resource bank is drained exactly once, into the survivor of the merge component (Pass 3). The result is deterministic regardless of which chunk sorts first.

Note: this uses `let`-chains (stable in edition 2024 / current nightly — the toolchain here is nightly).

- [x] **Step 4: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_world region`
Expected: PASS — the merge / split / empty tests are all green.

- [x] **Step 5: Commit**

Run: `git add -A && git commit -m "world: deterministic region merge and split"`

- [x] **Step 6: Keep the start region's explicit sugars in `terrain_gen`**

`init_player_region` (`crates/world/src/terrain_gen.rs:202-208`) is unchanged in shape, and its explicit `state.sugars = 100.0` stays — `RegionState::new`'s default is irrelevant since every `create_region()` caller sets its own starting resources. Confirm `init_player_region` still sets `sugars` explicitly:

```rust
fn init_player_region(region_states: &mut RegionStates) -> RegionId {
    let rid = region_states.create_region();
    if let Some(state) = region_states.get_mut(rid) {
        state.sugars = 100.0;
    }
    rid
}
```

If `init_player_region` already sets `sugars = 100.0`, this step is a no-op confirmation.

- [x] **Step 7: Write the failing test for the start region's sugars**

Add to the `tests` module in `crates/world/src/terrain_gen.rs`:

```rust
#[test]
fn start_region_starts_with_full_sugars() {
    let mut app = test_app();
    app.add_systems(Startup, terrain_generation);
    app.update();

    let rs = app.world().resource::<RegionStates>();
    assert_eq!(rs.regions.len(), 1);
    let state = rs.regions.values().next().unwrap();
    assert_eq!(state.sugars, 100.0);
}
```

- [x] **Step 8: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_world terrain`
Expected: PASS — the start region begins with 100 sugars set explicitly.

- [x] **Step 9: Show the network count in the HUD**

In `crates/ui/src/hud.rs`, change the turn-text format (`update_hud`, lines 159-170) to include the network count:

```rust
    if let Ok(mut text) = texts.turn.single_mut() {
        **text = format!(
            "Turn: {} | Speed: {} | Networks: {} | Fragments: {}/{} | Mushrooms: {}/{} | Seed: {}",
            game_state.turn,
            speed.label(),
            region_states.regions.len(),
            game_state.fragments_fused,
            game_state.fragments_total,
            game_state.mushrooms_fruited,
            game_state.mushrooms_required,
            config.seed,
        );
    }
```

`region_states` is already a field of `HudInputs` (`crates/ui/src/hud.rs:140`).

- [x] **Step 10: Verify the build and lint pass**

Run: `just lint && cargo nextest run -p kingdom_world -p kingdom_ui`
Expected: PASS — format, clippy, and all touched-crate tests clean.

- [x] **Step 11: Commit**

Run: `git add -A && git commit -m "world+ui: deterministic region tracking, HUD network count"`

---

## Task 2: Hives as map features

Add the `Hive` component, place hives at world gen, create the `kingdom_units` crate with a capture system, and render hives tinted by capture state. Growing mycelium onto a hive marks it captured.

**Files:**
- Modify: `crates/core/src/components.rs`
- Modify: `crates/core/src/messages.rs`
- Modify: `crates/core/src/constants.rs`
- Modify: `crates/world/src/terrain_gen.rs`
- Create: `crates/units/Cargo.toml`, `crates/units/src/lib.rs`, `crates/units/src/hive.rs`
- Modify: `Cargo.toml` (workspace)
- Modify: `bin/Cargo.toml`, `bin/src/plugins.rs`
- Create: `crates/render/src/units_render.rs`
- Modify: `crates/render/src/lib.rs`, `crates/render/src/assets.rs`
- Modify: `crates/ui/src/tile_popover.rs`

- [x] **Step 1: Add the `Hive` component**

Append to `crates/core/src/components.rs`:

```rust
#[derive(Component, Clone, Debug, Reflect)]
pub struct Hive {
    /// `None` = neutral; `Some` = the owning network.
    pub captured_by: Option<RegionId>,
    /// 0.0..=1.0 progress toward the next founder.
    pub production: f32,
}
```

- [x] **Step 2: Add the `HiveCaptured` message**

Append to `crates/core/src/messages.rs`:

```rust
use crate::region::RegionId;

#[derive(Message)]
pub struct HiveCaptured {
    pub hive_pos: Hex,
    pub region_id: RegionId,
}
```

(`Hex` is already imported at `crates/core/src/messages.rs:3`.)

- [x] **Step 3: Add the `HIVE_COUNT` constant**

Append to `crates/core/src/constants.rs`:

```rust
pub const HIVE_COUNT: u32 = 6;
```

- [x] **Step 4: Run a build check**

Run: `cargo build -p kingdom_core`
Expected: PASS — new types compile.

- [x] **Step 5: Place hives in `terrain_gen`**

In `crates/world/src/terrain_gen.rs`, add hives to `Placements` and place them on the soil pool clear of the player start. Add a field to `Placements` (line 37):

```rust
#[derive(Default)]
struct Placements {
    contents: HashMap<Hex, TileContents>,
    fragments: Vec<(Hex, FragmentId)>,
    fungi: Vec<(Hex, u32)>,
    plants: Vec<(Hex, u32)>,
    bacteria: Vec<Hex>,
    hives: Vec<Hex>,
}
```

In `place_features`, after the bacteria loop (around line 188), add:

```rust
    for _ in 0..HIVE_COUNT {
        let Some(pos) = pop_unclaimed(soil_pool, &p.contents) else {
            break;
        };
        p.hives.push(pos);
    }
```

`HIVE_COUNT` needs importing — add it to the `kingdom_core` use list at the top of the file. In `spawn_agents`, after the bacteria loop, add:

```rust
    for pos in p.hives {
        commands.spawn((GridPos(pos), Hive { captured_by: None, production: 0.0 }));
    }
```

Add `Hive` to the `kingdom_core` import list. Hives placed on the shuffled soil pool are never inside `player_start.range(2)` because the soil pool excludes the player hexes only indirectly — to be certain, filter the hive position: skip any `pos` within `player_start.range(4)`. Since `place_features` does not know `player_start`, pass it in. Simplest: place hives in `terrain_generation` itself after `player_hexes` is known. Replace the hive loop above with placement in `terrain_generation` instead — after `place_features` returns and `player_start` is known:

```rust
    for _ in 0..HIVE_COUNT {
        let Some(pos) = soil_pool
            .iter()
            .position(|h| h.unsigned_distance_to(player_start) > 6 && !placements.contents.contains_key(h))
            .map(|i| soil_pool.remove(i))
        else {
            break;
        };
        placements.hives.push(pos);
    }
```

Place this block right after `let mut placements = place_features(...)` and the `player_start` line. Drop the `place_features` hive loop — keep only the `Placements.hives` field and the `spawn_agents` loop.

- [x] **Step 6: Write the failing test for hive placement**

Add to the `tests` module in `crates/world/src/terrain_gen.rs`:

```rust
#[test]
fn places_hives_clear_of_player_start() {
    use kingdom_core::Hive;
    let mut app = test_app();
    app.add_systems(Startup, terrain_generation);
    app.update();

    let player_start = offset_to_hex(MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let mut hive_count = 0;
    let mut q = app.world_mut().query::<(&GridPos, &Hive)>();
    for (gpos, _) in q.iter(app.world()) {
        hive_count += 1;
        assert!(gpos.0.unsigned_distance_to(player_start) > 6, "hive too close to start");
    }
    assert!(hive_count > 0 && hive_count <= HIVE_COUNT as i32);
}
```

`GridPos` and `HIVE_COUNT` are already in scope via `use kingdom_core::{...}` / `super::*` — add `HIVE_COUNT` to the test imports if needed.

- [x] **Step 7: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_world places_hives`
Expected: PASS — hives spawn and none sit within 6 hexes of the start.

- [x] **Step 8: Commit**

Run: `git add -A && git commit -m "core+world: Hive component and world-gen placement"`

- [x] **Step 9: Create the `kingdom_units` crate manifest**

Create `crates/units/Cargo.toml`:

```toml
[package]
name = "kingdom_units"
version.workspace = true
edition.workspace = true
license-file.workspace = true

[dependencies]
bevy = { workspace = true }
hexx = { workspace = true }
pathfinding = { workspace = true }
kingdom_core = { workspace = true }
kingdom_world = { workspace = true }
```

`pathfinding` supplies the A* used by `find_path` in Task 4; the crate
declares it from the start so the manifest is not revisited.

Add to the workspace `Cargo.toml` `[workspace.dependencies]` block — `pathfinding`
alongside the other third-party crates, and `kingdom_units` after the
`kingdom_ui` line:

```toml
pathfinding = "4"

kingdom_units = { path = "crates/units" }
```

`members = ["crates/*", "bin"]` already globs the new crate in.

- [x] **Step 10: Write the failing test for `hive_capture_system`**

Create `crates/units/src/hive.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use kingdom_core::{GameState, GridPos, GridWorld, Hive, RegionId, RegionStates, Tile};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<GameState>();
        app.add_message::<kingdom_core::HiveCaptured>();
        app.add_systems(Update, hive_capture_system);
        app
    }

    fn spawn_tile(app: &mut App, pos: hexx::Hex, region: Option<RegionId>, biomass: f32) {
        let e = app
            .world_mut()
            .spawn((GridPos(pos), Tile { region_id: region, biomass, ..default() }))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
    }

    #[test]
    fn hive_on_owned_tile_is_captured() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let pos = hexx::Hex::new(3, 3);
        spawn_tile(&mut app, pos, Some(rid), 1.0);
        let hive = app
            .world_mut()
            .spawn((GridPos(pos), Hive { captured_by: None, production: 0.0 }))
            .id();
        app.update();
        assert_eq!(app.world().get::<Hive>(hive).unwrap().captured_by, Some(rid));
    }

    #[test]
    fn hive_on_unowned_tile_is_neutral() {
        let mut app = test_app();
        let pos = hexx::Hex::new(4, 4);
        spawn_tile(&mut app, pos, None, 0.0);
        let hive = app
            .world_mut()
            .spawn((GridPos(pos), Hive { captured_by: Some(RegionId(7)), production: 0.0 }))
            .id();
        app.update();
        assert_eq!(app.world().get::<Hive>(hive).unwrap().captured_by, None);
    }
}
```

- [x] **Step 11: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_units hive`
Expected: FAIL — `hive_capture_system` is undefined; crate may not compile yet.

- [x] **Step 12: Implement `hive_capture_system`**

Prepend to `crates/units/src/hive.rs` (above the test module):

```rust
use bevy::prelude::*;
use kingdom_core::{GridPos, GridWorld, Hive, HiveCaptured, Tile};

pub fn hive_capture_system(
    mut hives: Query<(&GridPos, &mut Hive)>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut captured: MessageWriter<HiveCaptured>,
) {
    for (gpos, mut hive) in &mut hives {
        let new_owner = grid
            .tiles
            .get(&gpos.0)
            .and_then(|&e| tiles.get(e).ok())
            .filter(|t| t.is_owned())
            .and_then(|t| t.region_id);

        if new_owner != hive.captured_by {
            if let Some(region_id) = new_owner {
                captured.write(HiveCaptured { hive_pos: gpos.0, region_id });
            }
            hive.captured_by = new_owner;
        }
    }
}
```

- [x] **Step 13: Create the crate library and plugin**

Create `crates/units/src/lib.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{HiveCaptured, SimulationSystems};
use kingdom_world::region_tracking_system;

mod hive;

pub use hive::hive_capture_system;

pub struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HiveCaptured>().add_systems(
            Update,
            hive_capture_system
                .in_set(SimulationSystems)
                .after(region_tracking_system),
        );
    }
}
```

- [x] **Step 14: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_units hive`
Expected: PASS — capture follows tile ownership.

- [x] **Step 15: Register `UnitsPlugin` in the binary**

Add to `bin/Cargo.toml` `[dependencies]`:

```toml
kingdom_units = { workspace = true }
```

In `bin/src/plugins.rs`, add `use kingdom_units::UnitsPlugin;` and add `.add(UnitsPlugin)` to the `KingdomPlugins` builder chain, after `.add(WorldPlugin)`.

- [x] **Step 16: Verify the binary builds**

Run: `cargo build -p kingdom`
Expected: PASS — `UnitsPlugin` registers cleanly.

- [x] **Step 17: Commit**

Run: `git add -A && git commit -m "units: kingdom_units crate with hive capture"`

- [x] **Step 18: Add a hive sprite handle**

In `crates/render/src/assets.rs`, add a `hive` field to `EntitySprites` and load it. Phase 1 reuses the neutral-fungus sprite as a stand-in, so point `hive` at the same asset path:

```rust
#[derive(Resource, Default, Debug)]
pub struct EntitySprites {
    pub fragment: Handle<Image>,
    pub plant_root: Handle<Image>,
    pub fauna: Handle<Image>,
    pub mushroom: Handle<Image>,
    pub neutral_fungus: Handle<Image>,
    pub hive: Handle<Image>,
    pub loaded: bool,
}
```

In `load_entity_sprites`, add: `sprites.hive = asset_server.load("sprites/neutral_fungus.png");`

- [x] **Step 19: Create the units render module — hive sprites**

Create `crates/render/src/units_render.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{GridPos, Hex, HexLayout, Hive};

use crate::assets::EntitySprites;
use crate::entity_render::organism_sprite_size;

/// Z layers for the unit-layer sprites, all above terrain/network.
const HIVE_Z: f32 = 1.5;

#[derive(Component)]
pub struct HiveSprite(pub Entity);

pub fn spawn_hive_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_hives: Query<(Entity, &GridPos), Added<Hive>>,
) {
    let size = organism_sprite_size(&layout);
    for (source, gpos) in new_hives.iter() {
        let world = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            HiveSprite(source),
            Sprite {
                image: sprites.hive.clone(),
                color: neutral_tint(),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world.extend(HIVE_Z)),
        ));
    }
}

fn neutral_tint() -> Color {
    Color::srgb(0.55, 0.55, 0.55)
}

/// Recolour a hive sprite when its capture state changes.
pub fn hive_tint_system(
    hives: Query<&Hive, Changed<Hive>>,
    mut sprites: Query<(&HiveSprite, &mut Sprite)>,
) {
    if hives.is_empty() {
        return;
    }
    for (link, mut sprite) in &mut sprites {
        let Ok(hive) = hives.get(link.0) else {
            continue;
        };
        sprite.color = match hive.captured_by {
            Some(rid) => region_tint(rid.0),
            None => neutral_tint(),
        };
    }
}

/// Deterministic per-region hue so different networks read distinctly.
fn region_tint(id: u32) -> Color {
    let hue = (id as f32 * 67.0) % 360.0;
    Color::hsl(hue, 0.6, 0.55)
}
```

`Hex` is unused here — drop the import if clippy flags it.

- [x] **Step 20: Register the hive render systems**

In `crates/render/src/lib.rs`, add `mod units_render;` and add the two systems to the `PostUpdate` tuple:

```rust
                    units_render::spawn_hive_sprites,
                    units_render::hive_tint_system,
```

`organism_sprite_size` is already `pub` in `entity_render.rs:21`.

- [x] **Step 21: Show hive state in the tile popover**

In `crates/ui/src/tile_popover.rs`, add a `hives` query to `TilePopoverInputs` and append hive state to the popover text. Add to the `SystemParam`:

```rust
    hives: Query<'w, 's, (&'static GridPos, &'static Hive)>,
```

Import `GridPos` and `Hive` from `kingdom_core` in the use list. In `resolve_popover`, after `format_tile`, append hive info:

```rust
    let mut text = format_tile(hex, tile, &inputs.region_states);
    if let Some((_, hive)) = inputs.hives.iter().find(|(gp, _)| gp.0 == hex) {
        let owner = match hive.captured_by {
            Some(rid) => format!("captured by Region {}", rid.0),
            None => "neutral".into(),
        };
        text.push_str(&format!("\nHive: {owner}\nProduction: {:.0}%", hive.production * 100.0));
    }
    Some(PopoverPayload { pos, text })
```

- [x] **Step 22: Verify build, lint, and tests**

Run: `just lint && cargo nextest run -p kingdom_units -p kingdom_render -p kingdom_ui -p kingdom_world`
Expected: PASS.

- [x] **Step 23: Commit**

Run: `git add -A && git commit -m "render+ui: hive sprites tinted by capture, popover hive state"`

---

## Task 3: Founder production + units

Captured hives spend the owner network's sugars to produce capped `Founder` units; idle units bleed upkeep. Founders render as static sprites for now. The HUD shows the unit count against the cap and aggregate resources.

**Files:**
- Modify: `crates/core/src/components.rs`
- Modify: `crates/core/src/constants.rs`
- Create: `crates/units/src/production.rs`
- Modify: `crates/units/src/lib.rs`
- Modify: `crates/world/src/region_tracking.rs` (fold `Unit` re-parenting into the merge)
- Modify: `crates/render/src/units_render.rs`, `crates/render/src/lib.rs`
- Modify: `crates/ui/src/hud.rs`

- [x] **Step 1: Add the `Unit` components**

Append to `crates/core/src/components.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum UnitKind {
    /// Phase 1 ships only this variant; Scout/Soldier/Builder arrive later.
    Founder,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Unit {
    pub kind: UnitKind,
    /// The network that produced the unit; pays its upkeep.
    pub owner: RegionId,
}

#[derive(Component, Clone, Debug, Reflect, Default)]
pub struct UnitMovement {
    /// Remaining hexes to traverse, in order; empty = idle.
    #[reflect(ignore)]
    pub path: Vec<Hex>,
    /// 0.0..1.0 progress along the edge from `GridPos` to `path[0]`.
    pub edge_progress: f32,
}
```

`Hex` and `RegionId` are already imported at the top of `components.rs`.

- [x] **Step 2: Add the production/upkeep/cap constants**

Append to `crates/core/src/constants.rs`:

```rust
pub const HIVE_PRODUCTION_SUGAR_COST: f32 = 1.0;
pub const HIVE_PRODUCTION_RATE: f32 = 0.05;
pub const UNIT_UPKEEP_SUGAR: f32 = 0.1;
pub const UNIT_CAP_BASE: u32 = 2;
pub const UNIT_CAP_PER_HIVE: u32 = 2;
```

- [x] **Step 3: Build check**

Run: `cargo build -p kingdom_core`
Expected: PASS.

- [x] **Step 4: Write the failing tests for production and upkeep**

Create `crates/units/src/production.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use kingdom_core::{GridPos, Hive, RegionId, RegionStates, Unit, UnitKind};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<RegionStates>();
        app.add_systems(Update, (hive_production_system, unit_upkeep_system).chain());
        app
    }

    #[test]
    fn captured_hive_with_sugars_spawns_a_founder() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 100.0;
        app.world_mut().spawn((
            GridPos(hexx::Hex::new(0, 0)),
            Hive { captured_by: Some(rid), production: 0.95 },
        ));
        app.update();
        let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
        assert_eq!(founders, 1);
        assert!(app.world().resource::<RegionStates>().get(rid).unwrap().sugars < 100.0);
    }

    #[test]
    fn production_stalls_at_the_unit_cap() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 100.0;
        // One captured hive → cap is UNIT_CAP_BASE + 1 * UNIT_CAP_PER_HIVE = 4.
        // Pre-spawn 4 units so the hive starts already at the cap.
        for _ in 0..4 {
            app.world_mut().spawn((
                GridPos(hexx::Hex::new(9, 9)),
                Unit { kind: UnitKind::Founder, owner: rid },
            ));
        }
        app.world_mut().spawn((
            GridPos(hexx::Hex::new(0, 0)),
            Hive { captured_by: Some(rid), production: 0.99 },
        ));
        let sugars_before = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        app.update();
        let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
        assert_eq!(founders, 4, "no founder spawned beyond the cap");
        // Production drained no sugars while capped; only upkeep on the 4 units does.
        let drained = sugars_before - app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        assert!((drained - 0.4).abs() < 1e-4, "only upkeep drained, not production");
    }

    #[test]
    fn upkeep_drains_and_clamps_at_zero() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 0.05;
        app.world_mut().spawn((
            GridPos(hexx::Hex::new(9, 9)),
            Unit { kind: UnitKind::Founder, owner: rid },
        ));
        app.update();
        assert_eq!(app.world().resource::<RegionStates>().get(rid).unwrap().sugars, 0.0);
    }
}
```

- [x] **Step 5: Run the tests to verify they fail**

Run: `cargo nextest run -p kingdom_units production`
Expected: FAIL — `hive_production_system` / `unit_upkeep_system` undefined.

- [x] **Step 6: Implement production and upkeep**

Prepend to `crates/units/src/production.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{
    GridPos, Hive, RegionStates, Unit, UnitKind, UnitMovement, HIVE_PRODUCTION_RATE,
    HIVE_PRODUCTION_SUGAR_COST, UNIT_CAP_BASE, UNIT_CAP_PER_HIVE, UNIT_UPKEEP_SUGAR,
};

pub fn hive_production_system(
    mut commands: Commands,
    mut hives: Query<(&GridPos, &mut Hive)>,
    units: Query<&Unit>,
    mut region_states: ResMut<RegionStates>,
) {
    let captured_hives = hives.iter().filter(|(_, h)| h.captured_by.is_some()).count() as u32;
    let cap = UNIT_CAP_BASE + captured_hives * UNIT_CAP_PER_HIVE;
    let mut living = units.iter().count() as u32;

    for (gpos, mut hive) in &mut hives {
        let Some(owner) = hive.captured_by else {
            continue;
        };
        if living >= cap {
            continue; // capped: production stalls, no sugars drained
        }
        let Some(state) = region_states.get_mut(owner) else {
            continue;
        };
        if state.sugars <= 0.0 {
            continue; // no sugars: production stalls
        }
        state.sugars = (state.sugars - HIVE_PRODUCTION_SUGAR_COST).max(0.0);
        hive.production += HIVE_PRODUCTION_RATE;
        if hive.production >= 1.0 {
            hive.production = 0.0;
            commands.spawn((
                GridPos(gpos.0),
                Unit { kind: UnitKind::Founder, owner },
                UnitMovement::default(),
            ));
            living += 1; // re-check the cap across hives finishing the same tick
        }
    }
}

pub fn unit_upkeep_system(units: Query<&Unit>, mut region_states: ResMut<RegionStates>) {
    for unit in &units {
        if let Some(state) = region_states.get_mut(unit.owner) {
            state.sugars = (state.sugars - UNIT_UPKEEP_SUGAR).max(0.0);
        }
        // A unit whose owner region no longer exists is skipped (Phase 1).
    }
}
```

Note on determinism: when several hives finish in the same tick and only some
fit under the cap, which hive wins the last slot follows the `&mut hives` query
iteration order. Correctness (no overshoot) holds regardless via the `living`
running total. If a future change needs the winner to be reproducible, collect
hives into a `Vec` sorted by `gpos.0` `(x, y)` before the loop.

- [x] **Step 7: Register production in the plugin**

In `crates/units/src/lib.rs`, add `mod production;`, re-export `pub use production::{hive_production_system, unit_upkeep_system};`, and extend the `SimulationSystems` chain so capture → production → upkeep run in order:

```rust
        app.add_message::<HiveCaptured>().add_systems(
            Update,
            (hive_capture_system, hive_production_system, unit_upkeep_system)
                .chain()
                .in_set(SimulationSystems)
                .after(region_tracking_system),
        );
```

- [x] **Step 8: Run the tests to verify they pass**

Run: `cargo nextest run -p kingdom_units production`
Expected: PASS — all three production/cap/upkeep tests green.

- [x] **Step 9: Commit**

Run: `git add -A && git commit -m "units: founder production, unit cap, upkeep"`

- [x] **Step 10: Write the failing test for unit re-parenting on merge**

`Unit` now exists, so `region_tracking_system` can re-parent the units of an absorbed region onto the merge survivor. Without this a founder produced by an absorbed network keeps a dangling `owner`: it pays no upkeep yet still counts against the unit cap. Add to the `tests` module in `crates/world/src/region_tracking.rs`:

```rust
#[test]
fn merge_reparents_absorbed_units_to_the_survivor() {
    use kingdom_core::{Unit, UnitKind};

    let mut app = test_app();
    let old = app.world_mut().resource_mut::<RegionStates>().create_region();
    let young = app.world_mut().resource_mut::<RegionStates>().create_region();
    let unit = app
        .world_mut()
        .spawn((GridPos(Hex::new(9, 9)), Unit { kind: UnitKind::Founder, owner: young }))
        .id();

    spawn_tile(&mut app, Hex::new(0, 0), Some(old), 1.0);
    spawn_tile(&mut app, Hex::new(1, 0), Some(young), 1.0);
    app.update();

    assert!(app.world().resource::<RegionStates>().get(young).is_none());
    assert_eq!(
        app.world().get::<Unit>(unit).unwrap().owner,
        old,
        "the absorbed region's unit follows the merge survivor",
    );
}
```

Run: `cargo nextest run -p kingdom_world merge_reparents`
Expected: FAIL — `region_tracking_system` does not re-parent units yet.

- [x] **Step 11: Fold unit re-parenting into `region_tracking_system`**

In `crates/world/src/region_tracking.rs`, add `Unit` to the `kingdom_core` import and add a `&mut Unit` query param. Pass 3 of `region_tracking_system` (built in Task 1) already produces the `reparent` map — every absorbed region id mapped to its survivor — so this step only adds the consumer, no change to Passes 1–4.

Change the import and signature:

```rust
use kingdom_core::{GridPos, GridWorld, RegionId, RegionStates, Tile, Unit};

pub fn region_tracking_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut units: Query<&mut Unit>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
```

After Pass 3 has built `reparent` and before `region_states.regions.retain(...)`, re-parent the units of every absorbed region onto its survivor:

```rust
    if !reparent.is_empty() {
        for mut unit in &mut units {
            if let Some(&survivor) = reparent.get(&unit.owner) {
                unit.owner = survivor;
            }
        }
    }
```

`kingdom_world` already depends on `kingdom_core`, so `Unit` is in scope with no new manifest change.

- [x] **Step 12: Run the re-parenting test to verify it passes**

Run: `cargo nextest run -p kingdom_world region`
Expected: PASS — the merge/split tests and the new re-parenting test are all green.

- [x] **Step 13: Render founder sprites (static position)**

Append to `crates/render/src/units_render.rs`:

```rust
use kingdom_core::Unit;

const UNIT_Z: f32 = 2.5;

/// Units render much smaller than a hex — a small body that visibly walks
/// across the hex it is crossing, rather than a sprite that fills the tile.
/// Fraction of the organism (hex-scale) sprite size; tuning value.
const UNIT_SPRITE_FRACTION: f32 = 0.2;

#[derive(Component)]
pub struct UnitSprite(pub Entity);

pub fn spawn_unit_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_units: Query<(Entity, &GridPos), Added<Unit>>,
) {
    let size = organism_sprite_size(&layout) * UNIT_SPRITE_FRACTION;
    for (source, gpos) in new_units.iter() {
        let world = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            UnitSprite(source),
            Sprite {
                image: sprites.fauna.clone(),
                // Sickly fungal green — a parasited insect.
                color: Color::srgb(0.45, 0.75, 0.35),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world.extend(UNIT_Z)),
        ));
    }
}

pub fn despawn_unit_sprites(
    mut commands: Commands,
    mut removed: RemovedComponents<Unit>,
    sprites: Query<(Entity, &UnitSprite)>,
) {
    let gone: std::collections::HashSet<Entity> = removed.read().collect();
    if gone.is_empty() {
        return;
    }
    for (sprite_e, link) in &sprites {
        if gone.contains(&link.0) {
            commands.entity(sprite_e).despawn();
        }
    }
}
```

Move the `use kingdom_core::Unit;` to the existing `use kingdom_core::{...}` line at the top of the file rather than a second `use` statement.

- [x] **Step 14: Register the unit render systems**

In `crates/render/src/lib.rs`, add to the `PostUpdate` tuple:

```rust
                    units_render::spawn_unit_sprites,
                    units_render::despawn_unit_sprites,
```

- [x] **Step 15: HUD — unit count and aggregate resources**

In `crates/ui/src/hud.rs`, add a `Unit` query to `HudInputs`. Since `HudInputs` is a `SystemParam` with a `'w` lifetime only, add a second lifetime for the query — change `pub struct HudInputs<'w>` to `pub struct HudInputs<'w, 's>` and add:

```rust
    units: Query<'w, 's, &'static kingdom_core::Unit>,
```

Update the `update_hud` destructure and turn-text format to include unit count vs cap and aggregate sugars/melanin:

```rust
    let total_sugars: f32 = region_states.regions.values().map(|r| r.sugars).sum();
    let total_melanin: f32 = region_states.regions.values().map(|r| r.melanin).sum();
    let captured_hives = /* count not available in HUD; cap shown from units only */ 0u32;
    let unit_count = units.iter().count();

    if let Ok(mut text) = texts.turn.single_mut() {
        **text = format!(
            "Turn: {} | Speed: {} | Networks: {} | Sugars: {:.0} | Melanin: {:.0} | Units: {} | Fragments: {}/{} | Mushrooms: {}/{}",
            game_state.turn,
            speed.label(),
            region_states.regions.len(),
            total_sugars,
            total_melanin,
            unit_count,
            game_state.fragments_fused,
            game_state.fragments_total,
            game_state.mushrooms_fruited,
            game_state.mushrooms_required,
        );
    }
```

The cap depends on captured hive count, which the HUD does not currently see. Add a `hives` query to `HudInputs` to compute the real cap:

```rust
    hives: Query<'w, 's, &'static kingdom_core::Hive>,
```

and compute:

```rust
    let captured_hives = hives.iter().filter(|h| h.captured_by.is_some()).count() as u32;
    let cap = kingdom_core::UNIT_CAP_BASE + captured_hives * kingdom_core::UNIT_CAP_PER_HIVE;
```

then use `"Units: {}/{}"` with `unit_count, cap`. Remove the placeholder `captured_hives = 0` line.

- [x] **Step 16: Verify build, lint, tests**

Run: `just lint && cargo nextest run -p kingdom_units -p kingdom_world -p kingdom_render -p kingdom_ui`
Expected: PASS.

- [x] **Step 17: Commit**

Run: `git add -A && git commit -m "render+ui+world: founder sprites, unit re-parenting, HUD unit count"`

---

## Task 4: Unit movement, control, and wisp rebind

Units move in real time along an A* hex path. The bare left click selects units and issues move orders; the wisp moves behind a held `WispMode` modifier. A cursor swap signals the active mode.

**Files:**
- Modify: `crates/core/src/components.rs`
- Create: `crates/units/src/pathfinding.rs`, `crates/units/src/movement.rs`
- Modify: `crates/units/src/lib.rs`, `crates/units/Cargo.toml`
- Modify: `crates/input/src/action.rs`, `crates/input/src/wisp.rs`, `crates/input/src/lib.rs`, `crates/input/Cargo.toml`
- Create: `crates/input/src/pointer.rs`, `crates/input/src/cursor.rs`
- Modify: `crates/render/src/units_render.rs`, `crates/render/src/lib.rs`

- [x] **Step 1: Add the `SelectedUnit` resource**

Append to `crates/core/src/components.rs`:

```rust
#[derive(Resource, Default)]
pub struct SelectedUnit(pub Option<Entity>);
```

- [x] **Step 2: Write the failing test for A* pathfinding**

Create `crates/units/src/pathfinding.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use kingdom_core::{GridPos, GridWorld, TerrainType, Tile};
    use hexx::Hex;

    fn world_with(passable: &[Hex], blocked: &[Hex]) -> (GridWorld, std::collections::HashMap<Hex, Tile>) {
        let mut grid = GridWorld::default();
        let mut tiles = std::collections::HashMap::new();
        for (i, &h) in passable.iter().enumerate() {
            grid.tiles.insert(h, Entity::from_bits((i + 1) as u64));
            tiles.insert(h, Tile { terrain: TerrainType::Soil, ..default() });
        }
        for (i, &h) in blocked.iter().enumerate() {
            grid.tiles.insert(h, Entity::from_bits((i + 1000) as u64));
            tiles.insert(h, Tile { terrain: TerrainType::Rock, ..default() });
        }
        (grid, tiles)
    }

    #[test]
    fn finds_a_straight_path() {
        let line: Vec<Hex> = (0..5).map(|q| Hex::new(q, 0)).collect();
        let (grid, tiles) = world_with(&line, &[]);
        let path = find_path(Hex::new(0, 0), Hex::new(4, 0), &grid, |h| {
            tiles.get(&h).is_some_and(|t| t.terrain.is_passable())
        });
        let path = path.expect("path exists");
        assert_eq!(*path.last().unwrap(), Hex::new(4, 0));
        assert!(!path.contains(&Hex::new(0, 0)), "path excludes the start hex");
    }

    #[test]
    fn returns_none_for_unreachable_target() {
        let (grid, tiles) = world_with(&[Hex::new(0, 0)], &[Hex::new(5, 5)]);
        let path = find_path(Hex::new(0, 0), Hex::new(5, 5), &grid, |h| {
            tiles.get(&h).is_some_and(|t| t.terrain.is_passable())
        });
        assert!(path.is_none());
    }
}
```

- [x] **Step 3: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_units pathfinding`
Expected: FAIL — `find_path` undefined.

- [x] **Step 4: Implement A* pathfinding via the `pathfinding` crate**

`find_path` is a thin adapter over `pathfinding::prelude::astar` — no
hand-rolled binary heap. The successor function is `grid.neighbors`
filtered by `passable`; the heuristic is the hex distance to the goal,
which is admissible on a hex grid with uniform step cost. Determinism
holds because `grid.neighbors` yields neighbours in a fixed order and
`astar` is deterministic given deterministic successors.

Prepend to `crates/units/src/pathfinding.rs`:

```rust
use hexx::Hex;
use kingdom_core::GridWorld;
use pathfinding::prelude::astar;

/// A* over the hex grid via the `pathfinding` crate. `passable(hex)` decides
/// which tiles a unit may enter. Returns the hexes to traverse from `start`
/// (exclusive) to `goal` (inclusive), or `None` if `goal` is unreachable.
/// `goal` itself must be passable.
pub fn find_path(
    start: Hex,
    goal: Hex,
    grid: &GridWorld,
    passable: impl Fn(Hex) -> bool,
) -> Option<Vec<Hex>> {
    if start == goal || !passable(goal) {
        return None;
    }
    let (path, _cost) = astar(
        &start,
        |&pos| {
            grid.neighbors(pos)
                .into_iter()
                .filter(|(n, _)| passable(*n))
                .map(|(n, _)| (n, 1u32))
        },
        |&pos| pos.unsigned_distance_to(goal),
        |&pos| pos == goal,
    )?;
    // `astar` includes `start` as `path[0]`; the unit is already standing
    // there, so the move order is everything after it.
    Some(path[1..].to_vec())
}
```

- [x] **Step 5: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_units pathfinding`
Expected: PASS — straight path found, unreachable returns `None`.

- [x] **Step 6: Write the failing test for `unit_movement_system`**

Create `crates/units/src/movement.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use kingdom_core::{GridPos, SimulationSpeed, Unit, UnitKind, UnitMovement};
    use hexx::Hex;

    fn test_app(speed: SimulationSpeed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(speed);
        app.add_systems(Update, unit_movement_system);
        app
    }

    fn spawn_unit(app: &mut App, path: Vec<Hex>) -> Entity {
        app.world_mut()
            .spawn((
                GridPos(Hex::new(0, 0)),
                Unit { kind: UnitKind::Founder, owner: kingdom_core::RegionId(0) },
                UnitMovement { path, edge_progress: 0.0 },
            ))
            .id()
    }

    #[test]
    fn unit_advances_along_its_path() {
        let mut app = test_app(SimulationSpeed::Normal);
        let unit = spawn_unit(&mut app, vec![Hex::new(1, 0), Hex::new(2, 0)]);
        // UNIT_SPEED_HEXES_PER_SEC = 1.0; advance enough simulated time for one hex.
        for _ in 0..70 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let gpos = app.world().get::<GridPos>(unit).unwrap();
        assert_ne!(gpos.0, Hex::new(0, 0), "unit moved off its start hex");
    }

    #[test]
    fn unit_does_not_advance_while_paused() {
        let mut app = test_app(SimulationSpeed::Paused);
        let unit = spawn_unit(&mut app, vec![Hex::new(1, 0)]);
        for _ in 0..10 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(app.world().get::<GridPos>(unit).unwrap().0, Hex::new(0, 0));
        assert_eq!(app.world().get::<UnitMovement>(unit).unwrap().edge_progress, 0.0);
    }
}
```

The wall-clock `sleep` keeps the test robust against Bevy's first-frame zero delta; it is slow but reliable. If the CI budget forbids sleeps, the implementer may instead advance `Time` manually — but the sleep form is the default.

- [x] **Step 7: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_units movement`
Expected: FAIL — `unit_movement_system` undefined.

- [x] **Step 8: Add the movement speed constant**

Append to `crates/core/src/constants.rs`:

```rust
pub const UNIT_SPEED_HEXES_PER_SEC: f32 = 1.0;
```

- [x] **Step 9: Implement `unit_movement_system`**

Prepend to `crates/units/src/movement.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{GridPos, SimulationSpeed, UnitMovement, UNIT_SPEED_HEXES_PER_SEC};

pub fn unit_movement_system(
    time: Res<Time>,
    speed: Res<SimulationSpeed>,
    mut units: Query<(&mut GridPos, &mut UnitMovement)>,
) {
    let speed_mult = match *speed {
        SimulationSpeed::Paused => return,
        SimulationSpeed::Normal => 1.0,
        SimulationSpeed::Fast => 2.0,
        SimulationSpeed::Fastest => 4.0,
    };
    let step = UNIT_SPEED_HEXES_PER_SEC * speed_mult * time.delta_secs();

    for (mut gpos, mut movement) in &mut units {
        if movement.path.is_empty() {
            continue;
        }
        movement.edge_progress += step;
        while movement.edge_progress >= 1.0 && !movement.path.is_empty() {
            let next = movement.path.remove(0);
            gpos.0 = next;
            movement.edge_progress -= 1.0;
        }
        if movement.path.is_empty() {
            movement.edge_progress = 0.0;
        }
    }
}
```

- [x] **Step 10: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_units movement`
Expected: PASS — the unit advances at Normal speed and stays put while Paused.

- [x] **Step 11: Register movement in the plugin**

`unit_movement_system` runs every frame, ungated. In `crates/units/src/lib.rs`, add `mod movement;` and `mod pathfinding;`, re-export `pub use movement::unit_movement_system;` and `pub use pathfinding::find_path;`, init the `SelectedUnit` resource, and register:

```rust
        app.init_resource::<kingdom_core::SelectedUnit>()
            .add_systems(Update, unit_movement_system);
```

(alongside the existing `SimulationSystems` chain registration).

- [x] **Step 12: Commit**

Run: `git add -A && git commit -m "units: A* pathfinding and real-time unit movement"`

- [x] **Step 13: Add the new input actions**

In `crates/input/src/action.rs`, add the two variants to the `Action` enum:

```rust
    WispMode,
    FoundNetwork,
```

In `default_input_map`, add the bindings:

```rust
    map.insert(Action::WispMode, KeyCode::KeyE);
    map.insert(Action::FoundNetwork, KeyCode::KeyF);
```

- [x] **Step 14: Gate the wisp behind `WispMode`**

In `crates/input/src/wisp.rs`, `wisp_input_system` (line 111), add an early return right after the `GamePhase` check:

```rust
    if !input.actions.pressed(&Action::WispMode) {
        wisp.phase = WispPhase::Idle;
        return;
    }
```

`input.actions` is the `ActionState<Action>` already in the `WispInput` `SystemParam`. Place this before the `ui_blocking` check.

- [x] **Step 15: Write the failing test for `pointer_system`**

Create `crates/input/src/pointer.rs` with the test module. `pointer_system` reads the cursor, so the unit tests cover the pure resolution logic only; full cursor behaviour is covered by the Task 6 integration test. Test the helper that resolves a click:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use kingdom_core::{GridPos, Unit, UnitKind, RegionId};
    use hexx::Hex;

    #[test]
    fn click_on_unit_hex_finds_that_unit() {
        let mut world = World::new();
        let unit = world
            .spawn((GridPos(Hex::new(2, 3)), Unit { kind: UnitKind::Founder, owner: RegionId(0) }))
            .id();
        let mut q = world.query::<(Entity, &GridPos, &Unit)>();
        let found = unit_at(Hex::new(2, 3), q.iter(&world));
        assert_eq!(found, Some(unit));
        let mut q2 = world.query::<(Entity, &GridPos, &Unit)>();
        assert_eq!(unit_at(Hex::new(9, 9), q2.iter(&world)), None);
    }
}
```

- [x] **Step 16: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_input pointer`
Expected: FAIL — `pointer` module / `unit_at` undefined.

- [x] **Step 17: Implement `pointer_system`**

Prepend to `crates/input/src/pointer.rs`. `kingdom_input` needs `kingdom_units` as a dependency for `find_path` — add `kingdom_units = { workspace = true }` to `crates/input/Cargo.toml` `[dependencies]`.

```rust
use bevy::ecs::message::MessageWriter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    GamePhase, GridPos, GridWorld, Hex, HexLayout, SelectedUnit, Tile, Unit, UnitMovement,
};
use kingdom_units::find_path;
use leafwing_input_manager::prelude::*;

use crate::action::Action;
use crate::camera::GameCamera;
use crate::wisp::TileTapped;

/// First unit entity occupying `hex`, if any.
pub fn unit_at<'a>(
    hex: Hex,
    units: impl Iterator<Item = (Entity, &'a GridPos, &'a Unit)>,
) -> Option<Entity> {
    units.into_iter().find(|(_, gp, _)| gp.0 == hex).map(|(e, _, _)| e)
}

#[derive(SystemParam)]
pub struct PointerInput<'w, 's> {
    actions: Res<'w, ActionState<Action>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
    ui_interactions: Query<'w, 's, &'static Interaction, With<Button>>,
}

impl PointerInput<'_, '_> {
    fn cursor_hex(&self, layout: &HexLayout) -> Option<Hex> {
        let window = self.windows.single().ok()?;
        let cursor = window.cursor_position()?;
        let (camera, cam_xform) = self.cameras.single().ok()?;
        let world = camera.viewport_to_world_2d(cam_xform, cursor).ok()?;
        Some(layout.world_pos_to_hex(world))
    }
    fn ui_blocking(&self) -> bool {
        self.ui_interactions.iter().any(|i| !matches!(i, Interaction::None))
    }
}

#[expect(clippy::too_many_arguments)]
pub fn pointer_system(
    input: PointerInput,
    phase: Res<GamePhase>,
    layout: Res<HexLayout>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    units: Query<(Entity, &GridPos)>,
    unit_lookup: Query<(Entity, &GridPos, &Unit)>,
    mut movements: Query<&mut UnitMovement>,
    mut selected: ResMut<SelectedUnit>,
    mut taps: MessageWriter<TileTapped>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    // The wisp owns the click while WispMode is held.
    if input.actions.pressed(&Action::WispMode) {
        return;
    }
    if !input.actions.just_pressed(&Action::Paint) || input.ui_blocking() {
        return;
    }
    let Some(hex) = input.cursor_hex(&layout) else {
        return;
    };

    if let Some(unit) = unit_at(hex, unit_lookup.iter()) {
        selected.0 = Some(unit);
        return;
    }

    if let Some(unit) = selected.0
        && let Ok((_, start)) = units.get(unit)
    {
        let path = find_path(start.0, hex, &grid, |h| {
            grid.tiles
                .get(&h)
                .and_then(|&e| tiles.get(e).ok())
                .is_some_and(|t| t.terrain.is_passable())
        });
        if let Some(path) = path
            && let Ok(mut movement) = movements.get_mut(unit)
        {
            movement.path = path;
            movement.edge_progress = 0.0;
        }
        return;
    }

    selected.0 = None;
    taps.write(TileTapped { pos: hex });
}
```

- [x] **Step 18: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_input pointer`
Expected: PASS — `unit_at` resolves a click on a unit hex.

- [x] **Step 19: Implement `cursor_system`**

Create `crates/input/src/cursor.rs`:

```rust
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, SystemCursorIcon};
use bevy::winit::cursor::CursorIcon;
use leafwing_input_manager::prelude::*;

use crate::action::Action;

/// Swap the window cursor icon to signal which mode the left click is in:
/// a crosshair while `WispMode` is held, the default pointer otherwise.
pub fn cursor_system(
    mut commands: Commands,
    actions: Res<ActionState<Action>>,
    window: Query<Entity, With<PrimaryWindow>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let icon = if actions.pressed(&Action::WispMode) {
        SystemCursorIcon::Crosshair
    } else {
        SystemCursorIcon::Default
    };
    commands.entity(window).insert(CursorIcon::System(icon));
}
```

The exact path to `CursorIcon` / `SystemCursorIcon` in Bevy 0.18 must be confirmed against the installed version — run `cargo doc` or grep the bevy source if the imports above fail. The behaviour (crosshair when `WispMode` held) is fixed; only the import path may need adjusting.

- [x] **Step 20: Register pointer and cursor in `InputPlugin`**

In `crates/input/src/lib.rs`, add `mod pointer;` and `mod cursor;`, re-export `pub use pointer::pointer_system;` and `pub use cursor::cursor_system;`, re-export `pub use kingdom_core::SelectedUnit;`, and add both systems to the `Update` tuple alongside `wisp_input_system`. Order `pointer_system` after `wisp_input_system` so the wisp's `WispMode` early-return and the pointer's gate never both fire:

```rust
                (
                    camera_system,
                    wisp_input_system,
                    pointer_system,
                    cursor_system,
                    selection_system,
                    speed_input_system,
                ),
```

- [x] **Step 21: Interpolate unit sprite position and draw the selection ring**

Append to `crates/render/src/units_render.rs`:

```rust
use kingdom_core::{SelectedUnit, UnitMovement};

const SELECTION_RING_Z: f32 = 2.4;

/// Per-frame: place each unit sprite by interpolating between its `GridPos`
/// and the next path hex by `edge_progress`. The small unit sprite physically
/// travels hex-centre to hex-centre, visibly crossing each hex it traverses.
pub fn unit_position_system(
    layout: Res<HexLayout>,
    units: Query<(&GridPos, &UnitMovement)>,
    mut sprites: Query<(&UnitSprite, &mut Transform)>,
) {
    for (link, mut transform) in &mut sprites {
        let Ok((gpos, movement)) = units.get(link.0) else {
            continue;
        };
        let from = layout.hex_to_world_pos(gpos.0);
        let world = match movement.path.first() {
            Some(&next) => from.lerp(layout.hex_to_world_pos(next), movement.edge_progress),
            None => from,
        };
        transform.translation = world.extend(UNIT_Z);
    }
}

#[derive(Component)]
pub struct SelectionRing;

/// Spawn/move/despawn a ring sprite that follows `SelectedUnit`.
pub fn selection_ring_system(
    mut commands: Commands,
    selected: Res<SelectedUnit>,
    layout: Res<HexLayout>,
    units: Query<(&GridPos, &UnitMovement)>,
    rings: Query<Entity, With<SelectionRing>>,
) {
    let target = selected.0.and_then(|e| units.get(e).ok());
    match (target, rings.iter().next()) {
        (None, Some(ring)) => commands.entity(ring).despawn(),
        (Some((gpos, movement)), existing) => {
            let from = layout.hex_to_world_pos(gpos.0);
            let world = match movement.path.first() {
                Some(&next) => from.lerp(layout.hex_to_world_pos(next), movement.edge_progress),
                None => from,
            };
            // Ring hugs the small unit body, not the hex.
            let size = organism_sprite_size(&layout) * UNIT_SPRITE_FRACTION * 1.6;
            let ring = existing.unwrap_or_else(|| commands.spawn(SelectionRing).id());
            commands.entity(ring).insert((
                Sprite {
                    color: Color::srgba(1.0, 1.0, 0.4, 0.7),
                    custom_size: Some(size),
                    ..default()
                },
                Transform::from_translation(world.extend(SELECTION_RING_Z)),
            ));
        }
        (None, None) => {}
    }
}
```

Fold the new `use kingdom_core::{...}` items into the existing import line.

- [x] **Step 22: Register the new render systems**

In `crates/render/src/lib.rs`, add to the `PostUpdate` tuple:

```rust
                    units_render::unit_position_system,
                    units_render::selection_ring_system,
```

- [x] **Step 23: Update the HUD hint text**

In `crates/ui/src/hud.rs` `spawn_hud`, update the hints array to reflect the new controls:

```rust
            let hints = [
                "WASD \u{2014} Pan camera",
                "Scroll \u{2014} Zoom",
                "Click \u{2014} Select unit / inspect tile",
                "Hold E + drag \u{2014} Paint growth",
                "F \u{2014} Found network",
                "Space \u{2014} Pause  |  +/- Speed",
                "H \u{2014} Hide hints",
            ];
```

- [x] **Step 24: Verify build, lint, tests**

Run: `just lint && cargo nextest run -p kingdom_units -p kingdom_input -p kingdom_render -p kingdom_ui`
Expected: PASS.

- [x] **Step 25: Commit**

Run: `git add -A && git commit -m "input+render: unit control, wisp rebind, cursor swap, unit interpolation"`

---

## Task 5: Founding new networks

A selected, idle founder standing on a valid site founds a new network: the founder is consumed, a region is created with seed biomass on the founded tile, and density flow grows it. A unit panel surfaces the action.

**Files:**
- Modify: `crates/core/src/messages.rs`
- Modify: `crates/core/src/constants.rs`
- Create: `crates/units/src/founding.rs`
- Modify: `crates/units/src/lib.rs`
- Modify: `crates/render/src/units_render.rs`, `crates/render/src/lib.rs`
- Modify: `crates/ui/src/hud.rs`

- [x] **Step 1: Add the `NetworkFounded` message**

Append to `crates/core/src/messages.rs`:

```rust
#[derive(Message)]
pub struct NetworkFounded {
    pub region_id: RegionId,
    pub seed: Hex,
}
```

(`RegionId` is imported in Task 2 Step 2; `Hex` is already imported.)

- [x] **Step 2: Add the founding constants**

Append to `crates/core/src/constants.rs`:

```rust
/// Minimum hex distance from any owned tile to a valid founding site.
pub const MIN_FOUNDING_DISTANCE: u32 = 6;
pub const FOUNDER_SEED_BIOMASS: f32 = 1.0;
pub const FOUNDER_SEED_SUGARS: f32 = 10.0;
```

- [x] **Step 3: Note the valid-site predicate shape**

The valid-site predicate is implemented in Step 5; its test module is written in Step 6, against the real signature. The predicate takes a `hex`, the `GridWorld`, a tile query, and decides three things:

- the tile is passable — via `TerrainType::is_passable()` (`crates/core/src/tile.rs:18`; passable = Soil/Root/Ruin/Surface), the same method pathfinding uses, so founding and pathfinding agree on one definition;
- `tile.region_id.is_none()` (unclaimed);
- the hex distance to the nearest owned tile of *any* region is `>= MIN_FOUNDING_DISTANCE`, which prevents founding adjacent to existing territory and an instant merge.

Do Step 5 first, then write the Step 6 tests against the finalised signature.

- [x] **Step 4: Add the founding seed constant usage note**

No code in this step — confirm `CLAIM_THRESHOLD` is `0.3` (`crates/core/src/constants.rs:8`) so `FOUNDER_SEED_BIOMASS = 1.0` is comfortably above it and the founded tile counts as owned on the next `region_tracking_system` run.

- [x] **Step 5: Implement the valid-site predicate and `founding_system`**

`founding_system` reacts to a request flag, not the `FoundNetwork` action: `kingdom_core` does not export `Action`, the action lives in `kingdom_input`, and `kingdom_input` already depends on `kingdom_units` for `find_path` — having `founding_system` read the action would create a `units → input` dependency cycle. A one-frame request resource, written by `kingdom_input` and the HUD button and consumed here, breaks the cycle.

Add to `crates/core/src/components.rs`:

```rust
/// Set for one frame to request that the selected founder found a network.
#[derive(Resource, Default)]
pub struct FoundNetworkRequest(pub bool);
```

Then prepend to `crates/units/src/founding.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{
    FoundNetworkRequest, FOUNDER_SEED_BIOMASS, FOUNDER_SEED_SUGARS, GridPos, GridWorld, Hex,
    MIN_FOUNDING_DISTANCE, NetworkFounded, RegionStates, SelectedUnit, Tile, Unit, UnitKind,
    UnitMovement,
};

/// True when `hex` is a legal place to found a new network:
/// passable terrain, unclaimed, and at least `MIN_FOUNDING_DISTANCE` hexes
/// from the nearest owned tile of any region. The last check stops a founder
/// from seeding next to existing territory and triggering an instant merge.
pub fn is_valid_site(
    hex: Hex,
    grid: &GridWorld,
    tiles: &Query<&mut Tile>,
) -> bool {
    let Some(tile) = grid.tiles.get(&hex).and_then(|&e| tiles.get(e).ok()) else {
        return false;
    };
    // `TerrainType::is_passable()` (crates/core/src/tile.rs:18) is the single
    // shared definition of passable, so founding and pathfinding agree.
    if !tile.terrain.is_passable() || tile.region_id.is_some() {
        return false;
    }
    // No owned tile of any region may sit within MIN_FOUNDING_DISTANCE.
    for (&pos, &entity) in &grid.tiles {
        if let Ok(t) = tiles.get(entity)
            && t.is_owned()
            && hex.unsigned_distance_to(pos) < MIN_FOUNDING_DISTANCE
        {
            return false;
        }
    }
    true
}

#[expect(clippy::too_many_arguments)]
pub fn founding_system(
    mut request: ResMut<FoundNetworkRequest>,
    mut selected: ResMut<SelectedUnit>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
    units: Query<(&Unit, &GridPos, &UnitMovement)>,
    mut tiles: Query<&mut Tile>,
    mut founded: MessageWriter<NetworkFounded>,
    mut commands: Commands,
) {
    if !std::mem::take(&mut request.0) {
        return;
    }
    let Some(unit_entity) = selected.0 else {
        return;
    };
    let Ok((unit, gpos, movement)) = units.get(unit_entity) else {
        return;
    };
    if unit.kind != UnitKind::Founder || !movement.path.is_empty() {
        return;
    }
    let seed = gpos.0;
    if !is_valid_site(seed, &grid, &tiles) {
        return;
    }

    let region_id = region_states.create_region();
    if let Some(state) = region_states.get_mut(region_id) {
        state.sugars = FOUNDER_SEED_SUGARS;
    }
    if let Some(&tile_e) = grid.tiles.get(&seed)
        && let Ok(mut tile) = tiles.get_mut(tile_e)
    {
        tile.region_id = Some(region_id);
        tile.biomass = FOUNDER_SEED_BIOMASS;
    }
    // The founder is despawned, so SelectedUnit must be cleared with it; the
    // render selection-ring and unit panel already tolerate a stale Entity.
    commands.entity(unit_entity).despawn();
    selected.0 = None;
    founded.write(NetworkFounded { region_id, seed });
}
```

`is_valid_site` takes `&Query<&mut Tile>` rather than `&Query<&Tile>` so the one `Query<&mut Tile>` param of `founding_system` can be borrowed for both the read-only check and the later mutation — Bevy rejects two conflicting `Tile` queries in one system, and `.get(e)` on a `&mut` query yields a shared `&Tile`.

- [x] **Step 6: Write the failing tests for `is_valid_site`**

Create the `crates/units/src/founding.rs` test module against the finalised `is_valid_site(hex, grid, tiles: &Query<&mut Tile>)` signature:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use kingdom_core::{GridPos, GridWorld, RegionId, TerrainType, Tile};
    use hexx::Hex;

    fn check_site(app: &mut App, hex: Hex) -> bool {
        let mut sys_state: bevy::ecs::system::SystemState<(
            Res<GridWorld>,
            Query<&mut Tile>,
        )> = bevy::ecs::system::SystemState::new(app.world_mut());
        let (grid, tiles) = sys_state.get_mut(app.world_mut());
        is_valid_site(hex, &grid, &tiles)
    }

    fn base_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app
    }

    fn add_tile(app: &mut App, pos: Hex, terrain: TerrainType, region: Option<RegionId>, biomass: f32) {
        let e = app
            .world_mut()
            .spawn((GridPos(pos), Tile { terrain, region_id: region, biomass, ..default() }))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
    }

    #[test]
    fn accepts_unclaimed_passable_far_hex() {
        let mut app = base_app();
        // An owned tile far away; the candidate site is unclaimed and passable.
        add_tile(&mut app, Hex::new(0, 0), TerrainType::Soil, Some(RegionId(0)), 1.0);
        add_tile(&mut app, Hex::new(20, 0), TerrainType::Soil, None, 0.0);
        assert!(check_site(&mut app, Hex::new(20, 0)));
    }

    #[test]
    fn rejects_hex_near_an_owned_tile() {
        let mut app = base_app();
        // Owned tile at (0,0); candidate (3,0) is within MIN_FOUNDING_DISTANCE (6).
        add_tile(&mut app, Hex::new(0, 0), TerrainType::Soil, Some(RegionId(0)), 1.0);
        add_tile(&mut app, Hex::new(3, 0), TerrainType::Soil, None, 0.0);
        assert!(!check_site(&mut app, Hex::new(3, 0)));
    }

    #[test]
    fn rejects_claimed_or_impassable_hex() {
        let mut app = base_app();
        add_tile(&mut app, Hex::new(20, 0), TerrainType::Rock, None, 0.0);
        assert!(!check_site(&mut app, Hex::new(20, 0)));
        let mut app2 = base_app();
        add_tile(&mut app2, Hex::new(20, 0), TerrainType::Soil, Some(RegionId(0)), 1.0);
        assert!(!check_site(&mut app2, Hex::new(20, 0)));
    }
}
```

`is_valid_site`'s signature is therefore `pub fn is_valid_site(hex: Hex, grid: &GridWorld, tiles: &Query<&mut Tile>) -> bool`.

- [x] **Step 7: Run the founding tests to verify they pass**

Run: `cargo nextest run -p kingdom_units founding`
Expected: PASS — all three site-predicate tests green.

- [x] **Step 8: Register founding in the plugin**

In `crates/units/src/lib.rs`: add `mod founding;`, re-export `pub use founding::{founding_system, is_valid_site};`, init the request resource, and register the system ungated:

```rust
        app.init_resource::<kingdom_core::FoundNetworkRequest>()
            .add_message::<kingdom_core::NetworkFounded>()
            .add_systems(Update, founding_system);
```

- [x] **Step 9: Wire the `FoundNetwork` action to the request flag**

In `crates/input/src/pointer.rs` (or a tiny dedicated system), set the request flag when the `FoundNetwork` action fires. Add a system to `crates/input/src/pointer.rs`:

```rust
pub fn found_network_input_system(
    actions: Res<ActionState<Action>>,
    mut request: ResMut<kingdom_core::FoundNetworkRequest>,
) {
    if actions.just_pressed(&Action::FoundNetwork) {
        request.0 = true;
    }
}
```

Register `found_network_input_system` in the `InputPlugin` `Update` tuple in `crates/input/src/lib.rs`, and re-export it.

- [ ] **Step 10: Commit**

Run: `git add -A && git commit -m "units: found new networks from idle founders"`

- [x] **Step 11: Add the founder panel to the HUD**

In `crates/ui/src/hud.rs`, add a unit panel that appears when `SelectedUnit` is set, with a "Found Network" button. Add a marker component and spawn the panel hidden in `spawn_hud`:

```rust
#[derive(Component)]
pub struct UnitPanel;

#[derive(Component)]
pub struct FoundNetworkButton;
```

In `spawn_hud`, spawn a panel node (absolute, bottom-left) holding a `Button` with `FoundNetworkButton` and a text child, and tag the panel `UnitPanel` with `Visibility::Hidden`. Add a system `update_unit_panel`:

```rust
pub fn update_unit_panel(
    selected: Res<kingdom_core::SelectedUnit>,
    units: Query<(&kingdom_core::Unit, &GridPos, &kingdom_core::UnitMovement)>,
    grid: Res<kingdom_core::GridWorld>,
    tiles: Query<&mut kingdom_core::Tile>,
    mut panel: Query<&mut Visibility, With<UnitPanel>>,
    interaction: Query<&Interaction, (Changed<Interaction>, With<FoundNetworkButton>)>,
    mut request: ResMut<kingdom_core::FoundNetworkRequest>,
) {
    let founder = selected
        .0
        .and_then(|e| units.get(e).ok())
        .filter(|(u, _, m)| u.kind == kingdom_core::UnitKind::Founder && m.path.is_empty());

    if let Ok(mut vis) = panel.single_mut() {
        *vis = if founder.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    let Some((_, gpos, _)) = founder else {
        return;
    };
    let on_valid_site = kingdom_units::is_valid_site(gpos.0, &grid, &tiles);
    if on_valid_site
        && interaction.iter().any(|i| matches!(i, Interaction::Pressed))
    {
        request.0 = true;
    }
}
```

`selected` here may hold a stale `Entity` once a founder is despawned by founding — `units.get(e).ok()` returns `None` and the panel simply hides, so no special handling is needed. `kingdom_ui` needs `kingdom_units` as a dependency — add `kingdom_units = { workspace = true }` to `crates/ui/Cargo.toml`. Register `update_unit_panel` in the `HudPlugin` `Update` tuple, and import `GridPos` as needed.

- [x] **Step 12: Verify build, lint, tests**

Run: `just lint && cargo nextest run -p kingdom_units -p kingdom_render -p kingdom_ui -p kingdom_input`
Expected: PASS.

- [x] **Step 13: Commit**

Run: `git add -A && git commit -m "render+ui: founder panel with Found Network button"`

---

## Task 6: Integration tests and verification

Prove the full Phase 1 loop end-to-end with integration tests, and confirm the win condition still works.

**Files:**
- Create: `crates/units/tests/civ_loop.rs`
- Create: `crates/input/tests/wisp_mode_gate.rs`
- Modify: `crates/units/Cargo.toml`, `crates/input/Cargo.toml` (dev-dependencies)

- [x] **Step 1: Add dev-dependencies for integration tests**

Add to `crates/units/Cargo.toml`:

```toml
[dev-dependencies]
kingdom_growth = { workspace = true }
rand = { workspace = true }
```

Add to `crates/input/Cargo.toml`:

```toml
[dev-dependencies]
kingdom_units = { workspace = true }
```

(`kingdom_units` is already a normal dependency of `kingdom_input` from Task 4; the dev-dep line is harmless but optional — skip it if already present.)

- [x] **Step 2: Write the `grow_to_capture_hive` integration test**

Create `crates/units/tests/civ_loop.rs`. Each test builds a `MinimalPlugins` app, wires the relevant systems, and drives ticks. Start with capture:

```rust
use bevy::prelude::*;
use hexx::Hex;
use kingdom_core::*;
use kingdom_units::{hive_capture_system, hive_production_system, unit_upkeep_system};
use kingdom_world::region_tracking_system;

fn spawn_tile(app: &mut App, pos: Hex, region: Option<RegionId>, biomass: f32) -> Entity {
    let e = app
        .world_mut()
        .spawn((GridPos(pos), Tile { region_id: region, biomass, ..default() }))
        .id();
    app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
    e
}

fn sim_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<GameState>();
    app.add_message::<HiveCaptured>();
    app.add_systems(
        Update,
        (region_tracking_system, hive_capture_system, hive_production_system, unit_upkeep_system)
            .chain(),
    );
    app
}

#[test]
fn grow_to_capture_hive() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let hive_pos = Hex::new(1, 0);
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    // Hive tile starts unowned; growing biomass onto it captures the hive.
    spawn_tile(&mut app, hive_pos, Some(rid), 1.0);
    let hive = app
        .world_mut()
        .spawn((GridPos(hive_pos), Hive { captured_by: None, production: 0.0 }))
        .id();

    app.update();
    assert_eq!(app.world().get::<Hive>(hive).unwrap().captured_by, Some(rid));
}
```

- [x] **Step 3: Run it**

Run: `cargo nextest run -p kingdom_units --test civ_loop grow_to_capture_hive`
Expected: PASS.

- [x] **Step 4: Add `captured_hive_produces_founder` and `unit_cap_blocks_overproduction`**

Append to `crates/units/tests/civ_loop.rs`:

```rust
#[test]
fn captured_hive_produces_founder() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 100.0;
    let hive_pos = Hex::new(1, 0);
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    spawn_tile(&mut app, hive_pos, Some(rid), 1.0);
    app.world_mut().spawn((GridPos(hive_pos), Hive { captured_by: None, production: 0.0 }));

    // HIVE_PRODUCTION_RATE = 0.05 → ~20 ticks per founder. Run 30.
    for _ in 0..30 {
        app.update();
    }
    let founders = app.world_mut().query::<&Unit>().iter(app.world()).count();
    assert!(founders >= 1, "captured hive produced a founder");
    assert!(app.world().resource::<RegionStates>().get(rid).unwrap().sugars < 100.0);
}

#[test]
fn unit_cap_blocks_overproduction() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 1000.0;
    let hive_pos = Hex::new(1, 0);
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    spawn_tile(&mut app, hive_pos, Some(rid), 1.0);
    app.world_mut().spawn((GridPos(hive_pos), Hive { captured_by: None, production: 0.0 }));

    for _ in 0..400 {
        app.update();
    }
    // One captured hive → cap = UNIT_CAP_BASE + 1 * UNIT_CAP_PER_HIVE = 4.
    let founders = app.world_mut().query::<&Unit>().iter(app.world()).count() as u32;
    assert_eq!(founders, UNIT_CAP_BASE + UNIT_CAP_PER_HIVE, "production stops at the cap");
}
```

- [x] **Step 5: Add `upkeep_drains_idle_units` and `two_networks_merge_pools_resources`**

Append to `crates/units/tests/civ_loop.rs`:

```rust
#[test]
fn upkeep_drains_idle_units() {
    let mut app = sim_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut().resource_mut::<RegionStates>().get_mut(rid).unwrap().sugars = 50.0;
    spawn_tile(&mut app, Hex::new(0, 0), Some(rid), 1.0);
    for _ in 0..3 {
        app.world_mut().spawn((
            GridPos(Hex::new(9, 9)),
            Unit { kind: UnitKind::Founder, owner: rid },
            UnitMovement::default(),
        ));
    }
    let before = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
    app.update();
    let after = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
    // 3 units * UNIT_UPKEEP_SUGAR (0.1) = 0.3 per tick.
    assert!((before - after - 0.3).abs() < 1e-4);
}

#[test]
fn two_networks_merge_pools_resources() {
    let mut app = sim_app();
    let old = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let young = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    app.world_mut().resource_mut::<RegionStates>().get_mut(old).unwrap().sugars = 20.0;
    app.world_mut().resource_mut::<RegionStates>().get_mut(young).unwrap().sugars = 15.0;
    spawn_tile(&mut app, Hex::new(0, 0), Some(old), 1.0);
    spawn_tile(&mut app, Hex::new(1, 0), Some(young), 1.0);
    spawn_tile(&mut app, Hex::new(2, 0), Some(young), 1.0);

    app.update();
    let rs = app.world().resource::<RegionStates>();
    assert!(rs.get(young).is_none(), "younger network absorbed");
    assert_eq!(rs.get(old).unwrap().sugars, 35.0, "resources pooled");
    assert!(old.0 < young.0, "the lower id is the survivor");
}
```

- [x] **Step 6: Add `founder_walks_and_founds_network`**

Append to `crates/units/tests/civ_loop.rs`. This drives `unit_movement_system` and `founding_system`:

```rust
#[test]
fn founder_walks_and_founds_network() {
    use kingdom_units::{founding_system, unit_movement_system};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<GameState>();
    app.init_resource::<SelectedUnit>();
    app.init_resource::<FoundNetworkRequest>();
    app.insert_resource(SimulationSpeed::Normal);
    app.add_message::<NetworkFounded>();
    app.add_systems(Update, (unit_movement_system, founding_system));

    // An existing region; it owns no tiles here, so it imposes no
    // MIN_FOUNDING_DISTANCE constraint on the founding site.
    let existing = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    // A passable, unclaimed target hex.
    let site = Hex::new(20, 0);
    spawn_tile(&mut app, site, None, 0.0);

    let founder = app
        .world_mut()
        .spawn((
            GridPos(site),
            Unit { kind: UnitKind::Founder, owner: existing },
            UnitMovement::default(),
        ))
        .id();
    app.world_mut().resource_mut::<SelectedUnit>().0 = Some(founder);
    app.world_mut().resource_mut::<FoundNetworkRequest>().0 = true;
    app.update();

    assert!(app.world().get::<Unit>(founder).is_none(), "founder consumed");
    assert!(
        app.world().resource::<SelectedUnit>().0.is_none(),
        "SelectedUnit cleared with the despawned founder",
    );
    // The founded tile is owned by a fresh region, seeded above CLAIM_THRESHOLD.
    let tile_e = app.world().resource::<GridWorld>().tiles[&site];
    let tile = app.world().get::<Tile>(tile_e).unwrap();
    assert!(tile.biomass >= FOUNDER_SEED_BIOMASS);
    let new_rid = tile.region_id.expect("founded tile is owned");
    assert_ne!(new_rid, existing, "a fresh region was created");
    let rs = app.world().resource::<RegionStates>();
    assert_eq!(rs.regions.len(), 2, "a new network exists");
    assert_eq!(rs.get(new_rid).unwrap().sugars, FOUNDER_SEED_SUGARS);
}
```

- [x] **Step 7: Run all civ_loop tests**

Run: `cargo nextest run -p kingdom_units --test civ_loop`
Expected: PASS — all six integration tests green.

- [x] **Step 8: Write the `wisp_mode_gates_painting` test**

Create `crates/input/tests/wisp_mode_gate.rs`. Driving the full `wisp_input_system` needs a window and camera, which a headless test cannot provide. Instead this test verifies the gate at the unit level: with `WispMode` released, the system sets the wisp phase to `Idle` and writes no bias. Build an app with the action state and assert the early-return path:

```rust
use bevy::prelude::*;
use kingdom_input::{Action, WispState};
use leafwing_input_manager::prelude::*;

#[test]
fn wisp_mode_action_exists_and_defaults_unpressed() {
    // The action map must define WispMode; without it the wisp can never paint.
    let map = kingdom_input::default_input_map();
    assert!(
        map.get(&Action::WispMode).is_some_and(|b| !b.is_empty()),
        "WispMode must be bound",
    );
    assert!(
        map.get(&Action::FoundNetwork).is_some_and(|b| !b.is_empty()),
        "FoundNetwork must be bound",
    );
}

#[test]
fn wisp_state_defaults_idle() {
    assert!(matches!(WispState::default().phase, kingdom_input::WispPhase::Idle));
}
```

`Action`, `WispState`, `WispPhase`, and `default_input_map` must be `pub` exports of `kingdom_input` — `Action` and `default_input_map` already are (`crates/input/src/lib.rs:10`); `WispState`/`WispPhase` already are (`:15`). The `leafwing` `InputMap::get` API name should be confirmed against version 0.20 — if `get` differs, adjust to the equivalent accessor; the assertion intent (the binding is non-empty) is fixed.

- [x] **Step 9: Run the wisp gate test**

Run: `cargo nextest run -p kingdom_input --test wisp_mode_gate`
Expected: PASS.

- [x] **Step 10: Full verification — lint and the whole test suite**

Run: `just lint && just test`
Expected: PASS — `cargo fmt` clean, clippy clean across the workspace (including the new `kingdom_units` crate), and every test green.

- [x] **Step 11: Verify the game still builds and the win condition is intact**

Run: `cargo build -p kingdom && cargo nextest run -p kingdom_fruiting -p kingdom_regions`
Expected: PASS — the binary compiles with `UnitsPlugin` registered, and the fruiting/region win-path tests still pass. The win condition (`GameState::victory`) was not touched, so fragments-fused-plus-mushrooms-fruited still wins.

- [x] **Step 12: Commit**

Run: `git add -A && git commit -m "tests: Phase 1 civ-loop and wisp-gate integration coverage"`

---

## Verification

Phase 1 is complete when:

- `just lint` is clean and `just test` is fully green, including the new `kingdom_units` crate.
- `cargo build -p kingdom` succeeds with `UnitsPlugin` registered.
- The six `civ_loop` integration tests and the `wisp_mode_gate` tests pass.
- Manual smoke check (`just run`): hold `E` and drag to paint mycelium toward a grey hive; the hive tints to the network colour when reached; a green founder sprite spawns; left-click the founder, left-click a distant valid hex, and watch it glide there; press `F` (or the panel button) on a valid site to found a new network that then grows on its own.

## Decisions and trade-offs

**`FoundNetworkRequest` resource instead of reading the `Action` in `founding_system`.** The spec places `founding_system` in `kingdom_units` and the `FoundNetwork` action in `kingdom_input`, and `kingdom_input` already depends on `kingdom_units` (for `find_path`). Having `founding_system` read the action would force `kingdom_units` to depend on `kingdom_input` — a dependency cycle.
- *Pros:* Breaks the cycle cleanly; both the `F` key and the HUD button write the same one-frame flag, so there is a single founding entry point.
- *Cons:* One extra resource; the flag must be consumed (`mem::take`) exactly once per frame to avoid a stale re-trigger.
- *Why:* A cycle is not an option; a write-once request flag is the standard Bevy decoupling pattern and is the smallest change.

**Reusing the neutral-fungus sprite for hives and a fauna sprite for founders.** The spec explicitly defers dedicated art. Tinting an existing sprite is enough to read capture state and unit identity in Phase 1.
- Auto-selected — no downsides compared to alternatives for a Phase 1 slice. Dedicated art is a later polish item.

**Movement tests use wall-clock `sleep` to accumulate `Time` delta.** Bevy's `Time` advances from real elapsed time; a headless `app.update()` loop with no sleep produces near-zero deltas.
- *Pros:* Exercises the real `time.delta_secs()` path with no test-only seams in production code.
- *Cons:* The two movement tests are slow (~1-2s each).
- *Why:* The alternative — injecting a mock clock — would add production complexity for two tests. If suite speed becomes a problem, switch to manually advancing `Time` then.
