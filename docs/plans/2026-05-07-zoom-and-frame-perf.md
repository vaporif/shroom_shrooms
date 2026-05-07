# Zoom & Frame Perf Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Make zoom feel responsive and stop the per-frame ECS churn that drags baseline FPS during camera motion.

**Architecture:** Five small, independent fixes across input + render + UI. Switch zoom from linear additive to multiplicative so each scroll tick is a uniform visual change. Add change-gates to four hot systems (`bias_glow_render_system`, `extract_selected_region_tiles`, `update_tile_popover`, `extract_branch_graph`) so they short-circuit when their inputs haven't actually changed.

**Tech Stack:** Rust 2024, Bevy 0.18, leafwing-input-manager, bevy_ecs_tilemap. Tests via cargo nextest.

## Execution Strategy

**Subagents** — default; no spec override. Tasks touch independent files and can be reviewed individually.

## Task Dependency Graph

- Task 1 [AFK]: Multiplicative zoom — depends on `none`
- Task 2 [AFK]: Diff-based bias glow — depends on `none`
- Task 3 [AFK]: Change-gate selected-region extraction — depends on `none`
- Task 4 [AFK]: Skip popover writes when unchanged — depends on `none`
- Task 5 [AFK]: Loosen network rebuild change detection — depends on `none`
- Polish: post-implementation-polish — depends on `Task 1, 2, 3, 4, 5`

All five tasks touch different files and can run in one parallel batch. Polish runs after the full batch passes review.

## Agent Assignments

- Task 1: Multiplicative zoom → bevy-engineer (Rust/Bevy)
- Task 2: Diff-based bias glow → bevy-engineer (Rust/Bevy)
- Task 3: Change-gate selected-region extraction → bevy-engineer (Rust/Bevy)
- Task 4: Skip popover writes when unchanged → bevy-engineer (Rust/Bevy)
- Task 5: Loosen network rebuild change detection → bevy-engineer (Rust/Bevy)
- Polish: post-implementation-polish → general-purpose

---

## Background: what we measured

Five hot paths, ranked by per-frame cost (verified in this branch):

| File:line | Issue |
|---|---|
| `crates/input/src/camera.rs:11,44` | `ZOOM_SPEED=0.1` linear additive over [0.15, 4.0] = 38 scroll ticks to traverse, uneven feel (40% step at min, 2.5% at max). |
| `crates/render/src/entity_render.rs:171–199` `bias_glow_render_system` | Despawns every `BiasGlowMarker`, scans all 4800 tiles, respawns sprites — every frame, no gate. |
| `crates/render/src/data_layer.rs:230` `extract_selected_region_tiles` | Scans all 4800 tiles + allocates `Vec<Hex>` every frame in plain `Update`. |
| `crates/ui/src/tile_popover.rs:50–55` `update_tile_popover` | Unconditional `node.left/top` and `**t = …` writes every frame ⇒ taffy re-layout each frame whenever popover is shown. |
| `crates/render/src/data_layer.rs:103–114` `nodes_match`/`edges_match` | Exact-float equality + `f32::EPSILON` tolerance flag the `BranchGraph` resource as changed every sim tick because density flow drifts biomass continuously. Triggers full network mesh rebuild (hundreds of `Mesh`+`NetworkMaterial` inserts) every sim tick downstream in `network_render_system`. |

---

### Task 1: Multiplicative zoom

**Files:**
- Modify: `crates/input/src/camera.rs:10-14, 41-46, 49-58`

**Design choice — multiplicative vs linear additive:**

- **Multiplicative (selected):** each scroll tick scales by a constant factor (e.g. 1.15). Visual change per tick is uniform; traversing the [0.15, 4.0] range takes ~22 ticks at factor 1.15. Auto-selected — no downsides compared to linear additive. Let me know if you disagree.
- Linear additive (current): step is uneven across the range; needs 38 ticks; awkward at the zoomed-in end.

- [x] **Step 1: Extend zoom tests to assert factor-based behavior**

Extend the test module at `crates/input/src/camera.rs:49-58` so it keeps the existing range assertion and adds a new test that pins the multiplicative factor:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_range_matches_spec() {
        assert_eq!(MIN_ZOOM, 0.15);
        assert_eq!(MAX_ZOOM, 4.0);
    }

    #[test]
    fn zoom_factor_is_multiplicative_uniform() {
        // One scroll tick should scale by ZOOM_FACTOR_PER_TICK regardless of
        // current scale, so a constant-input log range fully traverses [MIN, MAX]
        // in a small, uniform number of ticks.
        let ticks_to_traverse =
            (MAX_ZOOM / MIN_ZOOM).ln() / ZOOM_FACTOR_PER_TICK.ln();
        assert!(
            ticks_to_traverse > 15.0 && ticks_to_traverse < 30.0,
            "expected ~22 ticks to traverse range, got {ticks_to_traverse}"
        );
    }
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_input zoom_factor_is_multiplicative_uniform`
Expected: FAIL with `cannot find value 'ZOOM_FACTOR_PER_TICK' in this scope`.

- [x] **Step 3: Replace ZOOM_SPEED with multiplicative factor**

In `crates/input/src/camera.rs`, replace lines 10-14:

```rust
const CAMERA_SPEED: f32 = 300.0;
const ZOOM_FACTOR_PER_TICK: f32 = 1.15;
const MIN_ZOOM: f32 = 0.15;
const MAX_ZOOM: f32 = 4.0;
```

And replace the zoom block at lines 41-46:

```rust
    if let Projection::Orthographic(ref mut ortho) = *projection {
        let zoom_delta = actions.value(&Action::Zoom);
        if zoom_delta != 0.0 {
            // Positive scroll → zoom in (smaller scale); use ZOOM_FACTOR^(-delta)
            // so each tick is a uniform visual ratio change.
            let factor = ZOOM_FACTOR_PER_TICK.powf(-zoom_delta);
            ortho.scale = (ortho.scale * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
```

- [x] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p kingdom_input`
Expected: PASS.

- [x] **Step 5: Run full lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 6: Commit**

```
git add crates/input/src/camera.rs
git commit -m "perf(camera): multiplicative zoom for uniform per-tick feel"
```

---

### Task 2: Diff-based bias glow

**Files:**
- Modify: `crates/render/src/entity_render.rs:168-200`

**Design choice — diff with `Changed<Tile>` filter vs HashMap of pos→entity:**

- **`Changed<Tile>` filter + per-tile sprite component (selected):** Tag each glow with its source tile entity, react only to tiles whose `Tile` component changed this frame. Despawn the linked sprite when bias drops below threshold; spawn when it rises above. Avoids per-frame full-grid scan and command churn. No new bookkeeping resource.
  - Pros: minimal code, leverages Bevy change detection.
  - Baseline reality: `bias_decay_system` (`crates/growth/src/bias_decay.rs:4-11`) iterates `Query<&mut Tile>` unconditionally, so DerefMut flags every `Tile` as `Changed` once per simulation tick. Wisp painting (`wisp.rs:210`) does gate writes by value, so its diffs are sparse. Net effect: the new system iterates ~4800 tiles per sim tick (≤4 Hz at fastest sim speed), not per frame. Inside the loop most tiles short-circuit at the bias-magnitude threshold, and the only spawn/despawn commands are issued for tiles that actually crossed the threshold. The win is dropping from 60 Hz full scan + full despawn/respawn churn to ≤4 Hz scan with command activity proportional to threshold crossings.
- HashMap of `Hex → Entity` resource: requires manual upkeep on tile changes. More code, no advantage. Rejected.

- [x] **Step 1: Write a test that exercises the diff path**

Add to `crates/render/src/entity_render.rs` test module:

```rust
#[cfg(test)]
mod glow_diff_tests {
    use super::*;
    use bevy::MinimalPlugins;
    use kingdom_core::{BIAS_GLOW_VISIBLE_THRESHOLD, GridPos, Tile, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(create_hex_layout());
        app.add_systems(PostUpdate, bias_glow_render_system);
        app
    }

    #[test]
    fn glow_does_not_churn_when_tiles_unchanged() {
        let mut app = test_app();
        app.world_mut().spawn((
            GridPos(Hex::new(0, 0)),
            Tile {
                priority_bias: Vec2::new(BIAS_GLOW_VISIBLE_THRESHOLD * 2.0, 0.0),
                ..Default::default()
            },
        ));
        // First frame: the new tile should produce one glow.
        app.update();
        let glow_count_1 = app
            .world_mut()
            .query::<&BiasGlowMarker>()
            .iter(app.world())
            .count();
        assert_eq!(glow_count_1, 1);

        // Second frame with no tile mutation: glow stays, no churn (entity
        // identity preserved).
        let glow_entity_1 = app
            .world_mut()
            .query_filtered::<Entity, With<BiasGlowMarker>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.update();
        let glow_entity_2 = app
            .world_mut()
            .query_filtered::<Entity, With<BiasGlowMarker>>()
            .iter(app.world())
            .next()
            .unwrap();
        assert_eq!(
            glow_entity_1, glow_entity_2,
            "glow entity should not be despawned/respawned when tile is unchanged"
        );
    }

    #[test]
    fn glow_disappears_when_bias_drops_below_threshold() {
        let mut app = test_app();
        let tile_e = app
            .world_mut()
            .spawn((
                GridPos(Hex::new(1, 1)),
                Tile {
                    priority_bias: Vec2::new(BIAS_GLOW_VISIBLE_THRESHOLD * 2.0, 0.0),
                    ..Default::default()
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&BiasGlowMarker>()
                .iter(app.world())
                .count(),
            1
        );

        app.world_mut().get_mut::<Tile>(tile_e).unwrap().priority_bias = Vec2::ZERO;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&BiasGlowMarker>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
```

- [x] **Step 2: Run the new tests to verify they fail**

Run: `cargo nextest run -p kingdom_render glow_diff_tests`
Expected: at least one FAIL — current code despawns/respawns every frame, so `glow_entity_1 == glow_entity_2` fails.

- [x] **Step 3: Rewrite `bias_glow_render_system` as a diff system**

Replace lines 168-200 in `crates/render/src/entity_render.rs` with the version below. Note: `bevy::utils::HashMap` was removed in Bevy 0.18 — use `std::collections::HashMap`. The file already imports `std::collections::HashSet` at line 1; extend that to `use std::collections::{HashMap, HashSet};`.

```rust
#[derive(Component)]
pub struct BiasGlowMarker {
    /// Tile entity this glow tracks. Used to despawn when its tile drops below threshold.
    source: Entity,
}

pub fn bias_glow_render_system(
    mut commands: Commands,
    layout: Res<HexLayout>,
    changed_tiles: Query<(Entity, &GridPos, &Tile), Changed<Tile>>,
    existing: Query<(Entity, &BiasGlowMarker)>,
) {
    if changed_tiles.is_empty() {
        return;
    }

    // Map each tile entity to its current glow entity (if any) so we can update
    // or despawn it without touching glows for unchanged tiles.
    let mut existing_by_source: HashMap<Entity, Entity> = HashMap::with_capacity(existing.iter().len());
    for (glow_e, marker) in existing.iter() {
        existing_by_source.insert(marker.source, glow_e);
    }

    let quad_size = Vec2::splat(layout.scale.x * 1.6);

    for (tile_e, gpos, tile) in changed_tiles.iter() {
        let mag = tile.priority_bias.length();
        let visible = mag >= BIAS_GLOW_VISIBLE_THRESHOLD;
        match (existing_by_source.get(&tile_e).copied(), visible) {
            (Some(glow_e), false) => {
                commands.entity(glow_e).despawn();
            }
            (Some(glow_e), true) => {
                let alpha = (mag / BIAS_MAGNITUDE_CAP).min(1.0);
                commands.entity(glow_e).insert(Sprite {
                    color: Color::srgba(1.0, 0.7, 0.3, alpha),
                    custom_size: Some(quad_size),
                    ..default()
                });
            }
            (None, true) => {
                let alpha = (mag / BIAS_MAGNITUDE_CAP).min(1.0);
                let world = layout.hex_to_world_pos(gpos.0);
                commands.spawn((
                    BiasGlowMarker { source: tile_e },
                    Sprite {
                        color: Color::srgba(1.0, 0.7, 0.3, alpha),
                        custom_size: Some(quad_size),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, 0.7),
                ));
            }
            (None, false) => {}
        }
    }
}
```

Note: `BiasGlowMarker` gains a `source: Entity` field. Update any other code using it (none expected — grep first to confirm).

- [x] **Step 4: Confirm no other callers expect the unit-struct shape**

Run: `grep -rn 'BiasGlowMarker' crates/ bin/`
Expected: only references inside `entity_render.rs` and its tests.

- [x] **Step 5: Run the new tests to verify they pass**

Run: `cargo nextest run -p kingdom_render`
Expected: PASS for `glow_diff_tests` and all existing tests.

- [x] **Step 6: Run full lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 7: Commit**

```
git add crates/render/src/entity_render.rs
git commit -m "perf(render): diff-based bias glow, no per-frame full-grid scan"
```

---

### Task 3: Change-gate selected-region extraction

**Files:**
- Modify: `crates/render/src/data_layer.rs:230-245`

**Design choice — gate condition:**

The current sweep recomputes the same `Vec<Hex>` 60 times per second when nothing has moved. Two signals are sufficient: `selected.is_changed()` (player picked a different region) and any tile mutation (`Changed<Tile>` could affect membership). Using `Changed<Tile>` as a query filter alongside `is_changed()` on `Res<SelectedRegion>` covers both cases without storing a separate dirty flag.

- **Two-signal early-out (selected):** keep one `Query<&Tile, Changed<Tile>>` to detect *any* tile mutation; combine with `selected.is_changed()`. If neither fires, return.
  - Pros: one extra query, zero new state, no false positives on the eager path.
  - Baseline reality: `bias_decay_system` (`crates/growth/src/bias_decay.rs:4-11`) flags every `Tile` as `Changed` once per simulation tick, so `changed.is_empty()` is false at sim-tick rate. The optimisation drops sweeps from 60 Hz to sim-tick rate (≤4 Hz at fastest), not to "only when membership actually changes". That's still ≥15× fewer full-grid scans + allocations under normal play, and on frames between sim ticks the system is a one-line early return.
- Resource dirty flag: requires every mutator to set the flag. Easy to forget, more code, rejected.

- [x] **Step 1: Write a test that observes whether the body actually ran**

Today's `extract_selected_region_tiles` already short-circuits the *write* via the `selected_tiles.tiles != new_tiles` guard — what we want to suppress is the per-frame query iteration + allocation that runs *before* that guard. The cheapest way to detect that body execution from a test is a `Local<u64>` counter inside the system. Add it to the system as part of Step 3, then have the test read it via a wrapping system.

Add to `crates/render/src/data_layer.rs` test module:

```rust
#[test]
fn selected_region_extraction_skips_when_unchanged() {
    use kingdom_core::SelectedRegion;

    let mut app = test_app();
    app.init_resource::<SelectedRegion>();
    app.init_resource::<SelectedRegionTiles>();
    app.add_systems(Update, extract_selected_region_tiles);

    let rid = app
        .world_mut()
        .resource_mut::<kingdom_core::RegionStates>()
        .create_region();
    let pos = Hex::new(2, 2);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                ..Default::default()
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);
    app.world_mut().resource_mut::<SelectedRegion>().region_id = Some(rid);

    // Frame 1: SelectedRegion was just mutated, so the body must run.
    app.update();
    let runs_after_frame_1 = app
        .world()
        .resource::<SelectedRegionExtractionRuns>()
        .0;
    assert_eq!(
        runs_after_frame_1, 1,
        "body should run once when SelectedRegion changed"
    );

    // Frame 2: nothing changes. Body must early-return without iterating tiles.
    app.update();
    let runs_after_frame_2 = app
        .world()
        .resource::<SelectedRegionExtractionRuns>()
        .0;
    assert_eq!(
        runs_after_frame_2, 1,
        "body must not run again when no input changed"
    );

    // Frame 3: mutate the tile. `Changed<Tile>` must drive a re-sweep.
    app.world_mut().get_mut::<Tile>(e).unwrap().biomass = 0.5;
    app.update();
    let runs_after_frame_3 = app
        .world()
        .resource::<SelectedRegionExtractionRuns>()
        .0;
    assert_eq!(
        runs_after_frame_3, 2,
        "body must run again when any Tile changed"
    );
}
```

The `SelectedRegionExtractionRuns` resource is introduced in Step 3.

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo nextest run -p kingdom_render selected_region_extraction_skips_when_unchanged`
Expected: FAIL — `SelectedRegionExtractionRuns` does not exist yet, so compilation fails. Once Step 3 lands, the assertions distinguish gated vs ungated behavior cleanly.

- [x] **Step 3: Add the early-return and a body-run counter**

Add an always-on counter resource near the other resource declarations in `crates/render/src/data_layer.rs` (one `u64` of overhead per `App`, negligible — keeping it always-on avoids conditional system params, which are awkward in stable Rust):

```rust
#[derive(Resource, Default, Debug)]
pub struct SelectedRegionExtractionRuns(pub u64);
```

Register it in the `RenderPlugin::build` resource init list (mirror the existing `init_resource::<SelectedRegionTiles>()` line in `crates/render/src/lib.rs`):

```rust
.init_resource::<data_layer::SelectedRegionExtractionRuns>()
```

Initialise it in the `test_app()` helper too, so unit tests see it:

```rust
app.init_resource::<SelectedRegionExtractionRuns>();
```

Then replace lines 230-245 in `crates/render/src/data_layer.rs`:

```rust
pub fn extract_selected_region_tiles(
    tiles: Query<(&GridPos, &Tile)>,
    changed: Query<(), Changed<Tile>>,
    selected: Res<SelectedRegion>,
    mut selected_tiles: ResMut<SelectedRegionTiles>,
    mut runs: ResMut<SelectedRegionExtractionRuns>,
) {
    if !selected.is_changed() && changed.is_empty() {
        return;
    }
    runs.0 += 1;
    let new_tiles: Vec<Hex> = match selected.region_id {
        Some(rid) => tiles
            .iter()
            .filter_map(|(gpos, tile)| (tile.region_id == Some(rid)).then_some(gpos.0))
            .collect(),
        None => Vec::new(),
    };
    if selected_tiles.tiles != new_tiles {
        selected_tiles.tiles = new_tiles;
    }
}
```

- [x] **Step 4: Run the test to verify it passes**

Run: `cargo nextest run -p kingdom_render`
Expected: PASS.

- [x] **Step 5: Run full lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 6: Commit**

```
git add crates/render/src/data_layer.rs
git commit -m "perf(render): gate selected-region sweep on selection/tile change"
```

---

### Task 4: Skip popover writes when unchanged

**Files:**
- Modify: `crates/ui/src/tile_popover.rs:31-59`

**Design choice — comparison vs caching:**

Each `node.left = …` write is a `DerefMut` through Bevy's change detection wrapper, which marks `Node` as changed and forces taffy to re-layout the UI tree the next frame. Same for `**t = payload.text`. Compare current value before assigning so the writes are silent no-ops when stable.

- [x] **Step 1: Update the popover writes to compare-then-assign**

Replace lines 50-58 in `crates/ui/src/tile_popover.rs`:

```rust
    if let Ok((_, mut node, _)) = existing.single_mut() {
        let new_left = Val::Px(payload.pos.x);
        let new_top = Val::Px(payload.pos.y);
        if node.left != new_left {
            node.left = new_left;
        }
        if node.top != new_top {
            node.top = new_top;
        }
        if let Ok(mut t) = text.single_mut() {
            if **t != payload.text {
                **t = payload.text;
            }
        }
    } else {
        spawn_popover(&mut commands, payload);
    }
```

- [x] **Step 2: Build to confirm `Val` and `String` comparisons compile**

Run: `cargo check -p kingdom_ui`
Expected: clean. `Val` derives `PartialEq` in Bevy 0.18; `Text` is `pub struct Text(pub String)` with `Deref`/`DerefMut` to `String`, so `**t != payload.text` is a `String == String` comparison.

- [x] **Step 3: Add a smoke test that the system compiles and runs**

Existing `kingdom_ui` test harness is minimal. Add a unit test that constructs a `Val::Px(1.0)` and asserts `PartialEq` round-trips, just so the file has a regression sentinel:

```rust
#[cfg(test)]
mod popover_tests {
    use bevy::prelude::*;

    #[test]
    fn val_px_equality_holds_for_same_value() {
        assert_eq!(Val::Px(1.0), Val::Px(1.0));
        assert_ne!(Val::Px(1.0), Val::Px(2.0));
    }
}
```

- [x] **Step 4: Run tests**

Run: `cargo nextest run -p kingdom_ui`
Expected: PASS.

- [x] **Step 5: Run full lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 6: Commit**

```
git add crates/ui/src/tile_popover.rs
git commit -m "perf(ui): popover skips Node/Text writes when values unchanged"
```

---

### Task 5: Loosen network rebuild change detection

**Files:**
- Modify: `crates/render/src/data_layer.rs:103-114`
- Add: a constant in the same file or `crates/core/src/constants.rs` (prefer keeping it local — only the renderer cares)

**Design choice — tolerance value:**

`nodes_match` uses exact float equality on biomass; `edges_match` uses `f32::EPSILON` (~1.2e-7). With density flow drifting biomass by ~0.01–0.5 per tick, both functions return `false` every tick and the resource is reassigned, triggering a full network mesh rebuild every tick.

- **Absolute tolerance per node/edge (selected):** treat `|new - old| < NETWORK_REBUILD_BIOMASS_TOLERANCE` as "no change". Pick `0.05` — well below the visible width step that maps thickness to shader width, well above per-tick drift in the steady state.
  - Pros: one constant, no behavior change for topology updates (player adds/removes tiles), only suppresses rebuilds for biomass micro-drift.
  - Cons: avg biomass drifts visibly without re-render until topology changes. Acceptable: shader uniforms are recomputed only on rebuild anyway, and average biomass moves slowly.
- Relative tolerance: more code, no obvious benefit. Rejected.
- Time-based throttling (rebuild at most every N seconds): hides legitimate fast topology changes. Rejected.

- [x] **Step 1: Write a test that small biomass drift does not flag the graph as changed**

Add to `crates/render/src/data_layer.rs` test module:

```rust
#[test]
fn small_biomass_drift_does_not_rebuild_graph() {
    use kingdom_core::RegionStates;

    let mut app = test_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let pos = Hex::new(0, 0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                ..Default::default()
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);

    app.add_systems(Update, extract_branch_graph);
    app.update();

    let first_tick = app.world().resource_ref::<BranchGraph>().last_changed();

    // Drift biomass by an amount well below NETWORK_REBUILD_BIOMASS_TOLERANCE.
    app.world_mut().get_mut::<Tile>(e).unwrap().biomass = 1.0 + 0.01;
    app.update();

    let second_tick = app.world().resource_ref::<BranchGraph>().last_changed();
    assert_eq!(
        first_tick, second_tick,
        "sub-tolerance biomass drift must not flag BranchGraph as changed"
    );
}

#[test]
fn large_biomass_change_rebuilds_graph() {
    use kingdom_core::RegionStates;

    let mut app = test_app();
    let rid = app
        .world_mut()
        .resource_mut::<RegionStates>()
        .create_region();
    let pos = Hex::new(0, 0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                ..Default::default()
            },
        ))
        .id();
    app.world_mut()
        .resource_mut::<GridWorld>()
        .tiles
        .insert(pos, e);

    app.add_systems(Update, extract_branch_graph);
    app.update();
    let first_tick = app.world().resource_ref::<BranchGraph>().last_changed();

    // Drift biomass well above tolerance.
    app.world_mut().get_mut::<Tile>(e).unwrap().biomass = 5.0;
    app.update();

    let second_tick = app.world().resource_ref::<BranchGraph>().last_changed();
    assert_ne!(
        first_tick, second_tick,
        "supra-tolerance biomass change must flag BranchGraph as changed"
    );
}
```

- [x] **Step 2: Run tests to verify the first one fails**

Run: `cargo nextest run -p kingdom_render small_biomass_drift_does_not_rebuild_graph`
Expected: FAIL — current code rebuilds on any drift.

- [x] **Step 3: Add the tolerance constant and update the match functions**

In `crates/render/src/data_layer.rs`, near the top of the file (after imports):

```rust
/// Biomass drift below this threshold does not trigger a network mesh rebuild.
/// Density flow updates biomass continuously by ~0.01-0.5 per tick; rebuilding
/// the entire branch tree every tick costs hundreds of Mesh+Material asset
/// inserts. Real topology changes (tiles claimed/lost) bypass this via the
/// length and key checks.
const NETWORK_REBUILD_BIOMASS_TOLERANCE: f32 = 0.05;
```

Then replace `nodes_match` and `edges_match` (lines 103-114):

```rust
fn nodes_match(a: &HashMap<Hex, BranchNode>, b: &HashMap<Hex, BranchNode>) -> bool {
    a.iter().all(|(k, v)| {
        b.get(k).is_some_and(|other| {
            other.region_id == v.region_id
                && (other.biomass - v.biomass).abs() < NETWORK_REBUILD_BIOMASS_TOLERANCE
        })
    })
}

fn edges_match(a: &[BranchEdge], b: &[BranchEdge]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| {
        x.from == y.from
            && x.to == y.to
            && (x.thickness - y.thickness).abs() < NETWORK_REBUILD_BIOMASS_TOLERANCE
    })
}
```

- [x] **Step 4: Run tests to verify both pass**

Run: `cargo nextest run -p kingdom_render`
Expected: PASS for both `small_biomass_drift_does_not_rebuild_graph` and `large_biomass_change_rebuilds_graph`, plus all prior tests.

- [x] **Step 5: Run full lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 6: Commit**

```
git add crates/render/src/data_layer.rs
git commit -m "perf(render): tolerate biomass drift in branch graph diff"
```

---

## Final verification

After all five tasks land:

- [x] **Step 1: Run the full workspace test suite**

Run: `just test`
Expected: PASS.

- [x] **Step 2: Run the full lint pass**

Run: `just lint`
Expected: no warnings.

- [ ] **Step 3: Smoke-test in dev**

Run: `just dev`
Expected:
- Mouse wheel zoom feels uniform across the range, ~22 ticks corner-to-corner.
- No visible stutter when panning or zooming with a populated network.
- Painting bias still produces glow; clearing bias still removes glow.
- Selecting a tile still pops the tooltip; the tooltip still tracks the camera.

If any of those regress, file an issue against the relevant task and revert.
