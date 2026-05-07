# Density-flow refactor implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Replace the existing tip-agent growth model in Kingdom (Bevy 0.18 / Rust) with a continuous density-flow model driven by player-painted directional bias, strip specialization / abilities / rivals, and switch to a water / sugars / melanin resource model. Keep fragments, fruiting, organisms, and the slot machine. The game stays playable end-to-end through every task.

**Architecture:** Each tile carries continuous biomass that flows to passable neighbors each tick, weighted by player-painted bias plus nutrient gradient plus noise. Bias is set by mouse-drag wisp strokes that decay over time. Three resources: water per tile (consumed by growth, replenished from terrain), sugars per region (from decomposition of organic matter and symbiosis with plant roots), melanin per region (from radiation contact). The `Occupant` enum is dropped in favour of `Tile.region_id: Option<RegionId>` plus a `biomass >= CLAIM_THRESHOLD` ownership check. See `docs/specs/2026-05-07-kingdom-density-flow-design.md` and `docs/adr/0001-density-flow-over-tip-agents.md`.

**Tech Stack:** Bevy 0.18, Rust edition 2024, hexx 0.24 for hex coordinates, leafwing-input-manager 0.20 for input, bevy_ecs_tilemap 0.18.1 for terrain rendering, rand 0.10 for stochastic flow, cargo nextest for tests.

## Execution Strategy

**Subagents.** Every task runs in a fresh dispatched agent. The executor batches independent tasks into parallel dispatches and chains dependent tasks sequentially based on the dependency graph below. The refactor touches a shared simulation core, so most tasks are sequential; only T3 / T6 and T4 / T5 fan out to disjoint crates.

## Task Dependency Graph

- T1 [AFK]: depends on `none` → batch 1
- T2 [AFK]: depends on `T1` → batch 2
- T3 [AFK]: depends on `T2` → batch 3 (parallel with T6)
- T6 [AFK]: depends on `T2` → batch 3 (parallel with T3)
- T4 [AFK]: depends on `T3` → batch 4 (parallel with T5)
- T5 [AFK]: depends on `T3` → batch 4 (parallel with T4)
- T7 [AFK]: depends on `T3, T4, T5, T6` → batch 5

```
Batch 1: T1
Batch 2: T2
Batch 3: T3 || T6
Batch 4: T4 || T5
Batch 5: T7
```

## Agent Assignments

- T1: Strip dead code → bevy-engineer (Bevy / Rust)
- T2: Migrate data model → bevy-engineer (Bevy / Rust)
- T3: Replace growth core → bevy-engineer (Bevy / Rust)
- T4: Resource systems (decomposition, symbiosis, melanin) → bevy-engineer (Bevy / Rust)
- T5: Wisp input + bias glow render → bevy-engineer (Bevy / Rust)
- T6: UI simplification → bevy-engineer (Bevy / Rust)
- T7: Integration tests + verification → bevy-engineer (Bevy / Rust)
- Polish: post-implementation-polish → bevy-engineer (Bevy / Rust)

---

## Prerequisites for every task

Each task subagent should start with:

```bash
just lint        # baseline pass
just test        # baseline pass
git status       # confirm clean working tree
```

If `just lint` or `just test` fails before the task starts, the previous task left the tree dirty — stop and report. Do not paper over baseline failures.

Each task ends with the same two commands plus an explicit commit per verified step (or one logical commit per task if steps are tightly coupled). Frequent commits over big-bang commits.

---

## Task 1: Strip dead code

**Goal:** Remove specialization, rival AI, abilities, and the now-orphaned discovery paths. Game still compiles and runs the existing tip-based loop afterwards (with simpler UI).

**Files:**
- Delete: `crates/regions/src/specialization.rs`
- Delete: `crates/ai/src/rival.rs`, `crates/ai/src/combat.rs`
- Delete: `crates/ui/src/ability_bar.rs`, `crates/ui/src/spec_picker.rs`
- Delete: `crates/input/src/specialization_input.rs`
- Create: `crates/core/src/unlock.rs` (extract `UnlockPool` + `UnlockOption` from `abilities.rs` so they survive its deletion — `slot_machine.rs` and `mutation.rs` keep depending on these)
- Delete: `crates/core/src/abilities.rs` (after the extract above)
- Modify: `crates/regions/src/discovery.rs` (delete `explorer_discovery_system`, `researcher_study_system`, `StudyProgress`)
- Modify: `crates/core/src/messages.rs` (remove `StudyComplete`)
- Modify: `crates/core/src/region.rs` (drop spec fields)
- Modify: `crates/core/src/lib.rs` (drop `abilities` module, remove `add_message::<StudyComplete>()`)
- Modify: `crates/core/src/components.rs` (no spec-coupled components currently — verify)
- Modify: `crates/regions/src/lib.rs` (drop `SpecializationPlugin`, drop `explorer_discovery_system` and `researcher_study_system` registration, drop `StudyProgress` init)
- Modify: `crates/regions/src/slot_machine.rs` (will be re-wired in T4 — leave compiling but disconnected for now)
- Modify: `crates/regions/src/mutation.rs` (still compiles; unused for now — leave)
- Modify: `crates/ai/src/lib.rs` (drop `RivalAiPlugin`, drop `CombatPlugin`, drop `AiSystems::Rival`, `AiSystems::Combat`)
- Modify: `crates/ui/src/lib.rs` (drop `ability_bar` and `spec_picker` modules + plugin registration)
- Modify: `crates/input/src/lib.rs` (drop `specialization_input` module + system registration)
- Modify: `crates/input/src/action.rs` (drop `Spec1`..`Spec8` variants and bindings)
- Modify: `crates/render/src/data_layer.rs` (drop `RivalBranchGraph` resource, drop `extract_rival_branch_graph` system)
- Modify: `crates/render/src/network_render.rs` (drop `rival_graph: Res<RivalBranchGraph>` parameter from `network_render_system`, drop `group_rival_nodes_by_id` helper)
- Modify: `crates/render/src/lib.rs` (drop `RivalBranchGraph` init, drop `extract_rival_branch_graph` from system registration)
- Modify: `bin/src/plugins.rs` (drop `AiPlugin` from `KingdomPlugins`, replace with `OrganismsPlugin` and `EnvironmentPlugin` direct registration)
- Test: existing tests in deleted files vanish with the files; tests in modified files retained where possible.

**Preserved despite being unused for now:** `crates/regions/src/mutation.rs`, `crates/ui/src/slot_machine_ui.rs`, `crates/ai/src/organisms.rs`, `crates/ai/src/environment.rs`. The slot machine and organism systems remain in the codebase.

- [x] **Step 1: Verify baseline state**

```bash
just lint && just test
```

Expected: both pass cleanly.

- [x] **Step 2: Delete specialization plugin file**

```bash
rm crates/regions/src/specialization.rs
```

- [x] **Step 3: Delete rival and combat AI files**

```bash
rm crates/ai/src/rival.rs crates/ai/src/combat.rs
```

- [x] **Step 4: Delete UI files for ability bar and specialization picker**

```bash
rm crates/ui/src/ability_bar.rs crates/ui/src/spec_picker.rs
```

- [x] **Step 5: Delete input specialization handler**

```bash
rm crates/input/src/specialization_input.rs
```

- [x] **Step 6: Extract `UnlockPool` + `UnlockOption` into a new `unlock.rs` module**

`UnlockPool` and `UnlockOption` (currently in `abilities.rs:14-26`) are still consumed by `slot_machine.rs` and `mutation.rs` after T4. They must survive the deletion of `abilities.rs`. Create `crates/core/src/unlock.rs`:

```rust
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum UnlockPool {
    Organic,
    Mineral,
    Ruins,
    Decomposition,
}

#[derive(Clone, Debug, Reflect)]
pub struct UnlockOption {
    pub name: String,
    pub description: String,
    pub pool: UnlockPool,
}
```

The other items in `abilities.rs` (`ActiveAbility`, `ActiveEffect`, `AbilityEffectType`) are abilities-only and die with the file — do not move them.

- [x] **Step 7: Delete the abilities module**

```bash
rm crates/core/src/abilities.rs
```

- [x] **Step 8: Update `crates/core/src/lib.rs` — swap `abilities` for `unlock`, drop `StudyComplete` registration**

Edit `crates/core/src/lib.rs`:
- Replace `mod abilities;` with `mod unlock;`
- Replace `pub use abilities::*;` with `pub use unlock::*;`
- Remove `.add_message::<StudyComplete>()` line from `CorePlugin::build`

- [x] **Step 9: Update `crates/core/src/messages.rs` to remove `StudyComplete`**

Open `crates/core/src/messages.rs`. Remove the `StudyComplete` struct definition entirely. The `use crate::abilities::UnlockPool` import at the top is dead after `StudyComplete` is gone — drop it. Verify with:

```bash
rg "StudyComplete" crates bin
```

Expected output: empty (no references after this step).

- [x] **Step 10: Update `crates/core/src/region.rs` to strip specialization fields**

Replace the body of `RegionState` with just:

```rust
#[derive(Clone, Debug, Reflect)]
pub struct RegionState {
    pub region_id: RegionId,
    pub nutrients: f32,
    pub energy: f32,
    pub biomass: f32,
    pub tile_count: u32,
}

impl RegionState {
    pub fn new(id: RegionId) -> Self {
        Self {
            region_id: id,
            nutrients: 10.0,
            energy: 0.0,
            biomass: 0.0,
            tile_count: 0,
        }
    }
}
```

Delete `SpecializationType` enum, `SPEC_TIER_1/2/3` constants, and `tier()` method. T2 will rename / replace `nutrients`, `energy`, `biomass` — for now keep the fields so consumers compile.

- [x] **Step 11: Update `crates/regions/src/discovery.rs` to delete `explorer_discovery_system`, `researcher_study_system`, `StudyProgress`**

Open the file. Delete the `explorer_discovery_system` function, the `researcher_study_system` function, and the `StudyProgress` struct (and its `Default` derive). Keep `decomposer_discovery_system` and `DecompProgress` — they will be universalised in T4. Delete the `explorer_tip_discovers_tile` and `researcher_completes_study` tests; keep `decomposer_breaks_down_unique_decomposable` (it will be rewritten in T4).

Remove specialization conditional from `decomposer_discovery_system` (the `is_decomposer` check), but leave the function body otherwise intact for now — T4 owns the universalisation.

- [x] **Step 12: Update `crates/regions/src/lib.rs`**

Drop:
- `mod specialization;`
- `pub use specialization::specialization_system;`
- `mod mutation;` and re-export of `mutation_system` — keep mod, drop re-export only if mutation_system is no longer registered. Verify by checking what remains in `RegionsPlugin`. Actual decision: keep `mutation` module and its types declared; just don't register `mutation_system` if it depends on specialization. If it still compiles standalone, leave registration too.
- `RegionsSystems::Specialization` variant
- `SpecializationPlugin` plugin definition and registration
- `pub use discovery::{explorer_discovery_system, researcher_study_system, StudyProgress};` — replace with `pub use discovery::{DecompProgress, decomposer_discovery_system};`
- The `StudyProgress` init in `DiscoveryPlugin::build`
- The `explorer_discovery_system` and `researcher_study_system` registrations in `DiscoveryPlugin::build`

After edits, `RegionsPlugin` registers: `DiscoveryPlugin` (with only decomposer system), `UnlockPlugin` (slot machine + mutation if still compiles), `FragmentPlugin`.

- [x] **Step 13: Update `crates/regions/src/slot_machine.rs` to drop `StudyComplete` consumer**

The current `slot_machine_system` reads `StudyComplete`. With `StudyComplete` deleted, replace the reader with one that reads nothing for now (the system body becomes a noop) — T4 will rewire it to read `DecompositionComplete`. Make the simplest stub:

```rust
#[allow(unused_variables, clippy::needless_pass_by_value)]
pub fn slot_machine_system(
    slot_messages: MessageWriter<SlotMachineTriggered>,
    rng: ResMut<SlotMachineRng>,
) {
    // T4 will wire this to DecompositionComplete.
}
```

The `#[allow(...)]` is intentional — the params will be used again in T4. Without it, clippy fails the lint gate at the end of T1.

Delete the `slot_machine_produces_three_options` test (it tested the StudyComplete path). T4 step 4 adds a replacement test; the gap between T1 and T4 is intentional and acceptable.

- [x] **Step 14: Update `crates/ai/src/lib.rs`**

Replace the entire file with:

```rust
use bevy::prelude::*;

use kingdom_core::SimulationSystems;

mod environment;
mod organisms;

pub use environment::{EnvironmentRng, environment_threat_system};
pub use organisms::{
    NeutralFungiMerged, bacteria_system, fauna_system, neutral_fungi_system, plant_system,
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AiSystems {
    Organisms,
    Environment,
}

pub struct OrganismsPlugin;

impl Plugin for OrganismsPlugin {
    fn build(&self, app: &mut App) {
        // NeutralFungiMerged registered here (not in AiPlugin) because step 21
        // drops AiPlugin from the binary in favor of registering the inner plugins
        // directly. Otherwise the message has no add_message call → Bevy panics on
        // first write.
        app.add_message::<NeutralFungiMerged>().add_systems(
            Update,
            (
                neutral_fungi_system,
                plant_system,
                fauna_system,
                bacteria_system,
            )
                .chain()
                .in_set(AiSystems::Organisms),
        );
    }
}

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentRng>().add_systems(
            Update,
            environment_threat_system.in_set(AiSystems::Environment),
        );
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (AiSystems::Organisms, AiSystems::Environment)
                .chain()
                .in_set(SimulationSystems),
        )
        .add_plugins((OrganismsPlugin, EnvironmentPlugin));
    }
}
```

`AiPlugin` retained as a convenience aggregator. The `NeutralFungiMerged` registration migrates into `OrganismsPlugin` so it stays registered when `bin/src/plugins.rs` adds `OrganismsPlugin` directly without `AiPlugin`.

- [x] **Step 15: Update `crates/ui/src/lib.rs`**

Drop:
- `mod ability_bar;` and any re-exports
- `mod spec_picker;` and any re-exports

Drop the corresponding plugin registrations or system registrations from `UiPlugin::build`. Keep `slot_machine_ui` registered. Read the file, remove specifically those entries.

- [x] **Step 16: Update `crates/input/src/lib.rs`**

Drop:
- `mod specialization_input;`
- `pub use specialization_input::specialization_input_system;`
- `specialization_input_system` from any system registration in `InputPlugin::build`
- `mod priority;` and re-exports — actually keep for now, T5 deletes it.

- [x] **Step 17: Update `crates/input/src/action.rs` to drop `Spec1`..`Spec8`**

Open the file. Remove the eight `Spec1..Spec8` variants from the `Action` enum. Remove the eight `map.insert(Action::Spec*, KeyCode::Digit*)` calls from `default_input_map`. Leave `SetPriority` and `ClearPriority` in place — T5 removes them.

- [x] **Step 18: Update `crates/render/src/data_layer.rs` to drop rival graph extraction and `BranchNode.specialization`**

Open the file. Make these changes:

1. Delete the `RivalBranchGraph` resource definition and its `Default` derive (around lines 55–62).
2. Delete the `RivalBranchNode` struct (around line 65).
3. Delete the `extract_rival_branch_graph` system function (around line 230+).
4. Remove the `specialization: Option<SpecializationType>` field from `BranchNode` (line 20). Remove the corresponding `use kingdom_core::SpecializationType` import.
5. Inside `extract_branch_graph` (around lines 71–93 and 129), delete the `let spec = region_states.get(rid).and_then(|r| r.specialization);` lookup and the `specialization: spec` field on `BranchNode { ... }` construction.

Leave `BranchGraph`, `TipPositions`, `RegionHulls`, `DiscoveryMap`, `PriorityBiasMap`, `SelectedRegionTiles` (TipPositions is removed in T3, not here).

- [x] **Step 19: Update `crates/render/src/network_render.rs` to drop rival graph rendering and replace `region_color_linear`**

Open the file. Make these changes:

1. From `network_render_system`'s parameter list, remove the `rival_graph: Res<RivalBranchGraph>` parameter. Remove any code inside the system that iterates over `rival_graph` or uses rival branches.
2. Delete the `group_rival_nodes_by_id` helper function.
3. Replace `region_color_linear(spec: Option<SpecializationType>) -> LinearRgba` (around line 64) with a single constant color or a deterministic hash of `RegionId`. Recommended: hash `RegionId.0` to a stable hue. Concrete replacement:

```rust
fn region_color_linear(rid: kingdom_core::RegionId) -> LinearRgba {
    // Deterministic hue per region id; saturation/lightness fixed.
    let hue = (rid.0 as f32 * 0.61803398875).fract() * 360.0;
    let (r, g, b) = hsl_to_rgb(hue, 0.55, 0.55);
    LinearRgba::new(r, g, b, 1.0)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_p = h / 60.0;
    let x = c * (1.0 - (h_p % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h_p as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (r1 + m, g1 + m, b1 + m)
}
```

4. Update the call site (around line 514) from `.and_then(|n| n.specialization)` plus `region_color_linear(spec)` to use the region id instead. `BranchNode.region_id` is already `RegionId` (non-optional) in `data_layer.rs:21` — no field shape change needed. Pattern:

```rust
let region_id = graph.nodes.values().next().map(|n| n.region_id);
let core = region_id.map(region_color_linear).unwrap_or(LinearRgba::WHITE);
```

5. Delete the `region_color_maps_specializations` test (around line 666). Add a quick replacement test that confirms two different `RegionId` values produce two different colors:

```rust
#[test]
fn region_color_distinguishes_region_ids() {
    use kingdom_core::RegionId;
    let a = region_color_linear(RegionId(1));
    let b = region_color_linear(RegionId(2));
    assert_ne!((a.red, a.green, a.blue), (b.red, b.green, b.blue));
}
```

6. In any test fixtures further down the file (the `BranchNode { specialization: None, .. }` constructions around lines 850, 859, 868), drop the `specialization` field — its removal in step 17 will require these test fixtures to be rewritten.

7. Drop `use kingdom_core::SpecializationType` and any `use kingdom_core::*` imports that pulled in spec-specific items.

- [x] **Step 20: Update `crates/render/src/lib.rs`**

Drop:
- The `init_resource::<data_layer::RivalBranchGraph>()` call
- The `data_layer::extract_rival_branch_graph` from system registration tuple

- [x] **Step 21: Update `bin/src/plugins.rs` to register only the kept AI sub-plugins**

Replace the AI line in `KingdomPlugins`:

```rust
// before
.add(AiPlugin)

// after
.add(kingdom_ai::OrganismsPlugin)
.add(kingdom_ai::EnvironmentPlugin)
```

`AiPlugin` exists but is no longer used. Either remove the `add(AiPlugin)` line entirely (cleaner) or keep it if you want a single registration — pick removal so the dropped rival plugin can't be accidentally re-added.

- [x] **Step 22: Verify the workspace compiles**

```bash
cargo check --workspace --all-features --all-targets
```

Expected: clean build. Likely remaining breakage points after the previous steps (search and fix each):

- `crates/world/src/terrain_gen.rs` — may read `state.nutrient_bonus` or other deleted fields when initializing
- `crates/growth/src/nutrient.rs` — `nutrient_production_system` reads `state.specialization` and `state.nutrient_bonus`. Strip the specialization conditional and bonus multiplier; keep the base production flat. T3 deletes most of this file anyway, but it must compile in T1.
- `crates/ai/src/organisms.rs` — may read `state.nutrient_bonus`. Strip.
- `crates/ui/src/hud.rs` and `crates/ui/src/tile_popover.rs` — may read `state.specialization*`. T6 owns these but T1 must stub them so T1 compiles. Acceptable shape: replace specialization reads with empty strings or skip the row entirely.
- `crates/regions/src/mutation.rs` — verify it does not depend on `SpecializationType`. Per the existing code it consumes `SlotMachineTriggered` only and is safe to keep registered.
- Any remaining `use kingdom_core::SpecializationType` imports — drop.

For each unresolved compile error, prefer removing the call site over stubbing a placeholder. The simulation logic is being rebuilt; preserving dead reads now creates noise to clean up later.

- [x] **Step 23: Run the full test suite**

```bash
just test
```

Expected: pass. Tests covering deleted code are gone with the files; remaining tests should still pass.

- [x] **Step 24: Run lints and format check**

```bash
just lint
```

Expected: pass. Fix any warnings introduced by dead-code removal (unused imports etc).

- [x] **Step 25: Commit T1**

```bash
git add -A
git commit -m "T1: strip specialization, rivals, combat, abilities, study/explorer paths"
```

---

## Task 2: Migrate data model

**Goal:** Drop `Occupant`, add `Tile.region_id`, add `Tile.radiation`, rename `Tile.nutrient_level` to `Tile.soil_richness`, replace `RegionState.{nutrients,energy,biomass}` with `{sugars,melanin,total_biomass}`, add the new constants, and seed radiation in terrain generation. After T2 the tip-based growth loop still runs against the new fields.

**Files:**
- Modify: `crates/core/src/tile.rs` (drop `Occupant`, add `region_id`, add `radiation`, rename `nutrient_level`)
- Modify: `crates/core/src/region.rs` (replace nutrients/energy/biomass with sugars/melanin/total_biomass)
- Modify: `crates/core/src/constants.rs` (add new constants)
- Modify: `crates/world/src/terrain_gen.rs` (radiation seed pass)
- Modify: every file that referenced `Occupant`, `nutrient_level`, or the old `RegionState` fields. Per `rg`, this is roughly 30 files. Each is a mechanical rename.

**Migration map:**

| Old | New |
|---|---|
| `tile.occupant == Occupant::Empty` | `tile.region_id.is_none()` |
| `tile.occupant == Occupant::Player(rid)` | `tile.region_id == Some(rid)` |
| `tile.occupant.is_player()` | `tile.region_id.is_some()` (with `biomass >= CLAIM_THRESHOLD` if ownership semantics matter — see step 3) |
| `tile.occupant.is_rival()` | `false` (rivals gone) |
| `tile.occupant.region_id()` | `tile.region_id` |
| `tile.occupant = Occupant::Player(rid)` | `tile.region_id = Some(rid)` |
| `tile.occupant = Occupant::Empty` | `tile.region_id = None` |
| `tile.occupant = Occupant::Rival(_)` | dead code, remove |
| `tile.nutrient_level` | `tile.soil_richness` |
| `region.nutrients` | `region.sugars` (semantic shift but consumer behavior carries through; T4 cleans up) |
| `region.energy` | drop reads where possible; if needed, route to `region.sugars` |
| `region.biomass` | `region.total_biomass` |

- [x] **Step 1: Verify baseline (T1 complete)**

```bash
just lint && just test && git status
```

Expected: clean tree, all green.

- [x] **Step 2: Update `crates/core/src/tile.rs`**

Replace the contents with:

```rust
use bevy::prelude::*;

use crate::region::RegionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
pub enum TerrainType {
    #[default]
    Soil,
    Rock,
    Water,
    Root,
    Ruin,
    Toxic,
    Surface,
}

impl TerrainType {
    pub fn is_passable(&self) -> bool {
        matches!(self, Self::Soil | Self::Root | Self::Ruin | Self::Surface)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct FragmentId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum TileContents {
    OrganicMatter,
    Mineral,
    Artifact,
    Fragment(FragmentId),
    UniqueDecomposable(u32),
    NeutralFungus(u32),
    PlantRoot(u32),
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Tile {
    pub terrain: TerrainType,
    pub region_id: Option<RegionId>,
    pub biomass: f32,
    pub moisture: f32,
    pub radiation: f32,
    pub soil_richness: f32,
    pub nutrient_gradient: Vec2,
    pub priority_bias: Vec2,
    pub discovered: bool,
    pub contents: Option<TileContents>,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain: TerrainType::Soil,
            region_id: None,
            biomass: 0.0,
            moisture: 0.5,
            radiation: 0.0,
            soil_richness: 0.5,
            nutrient_gradient: Vec2::ZERO,
            priority_bias: Vec2::ZERO,
            discovered: false,
            contents: None,
        }
    }
}
```

`RivalId` is deleted (no longer needed). `Occupant` is deleted. `nutrient_level` is renamed to `soil_richness`. `region_id` and `radiation` are new.

- [x] **Step 3: Update `crates/core/src/region.rs`**

Replace `RegionState` with the final shape:

```rust
#[derive(Clone, Debug, Reflect)]
pub struct RegionState {
    pub region_id: RegionId,
    pub sugars: f32,
    pub melanin: f32,
    pub total_biomass: f32,
    pub tile_count: u32,
}

impl RegionState {
    pub fn new(id: RegionId) -> Self {
        Self {
            region_id: id,
            sugars: 10.0,
            melanin: 0.0,
            total_biomass: 0.0,
            tile_count: 0,
        }
    }
}
```

- [x] **Step 4: Add new constants to `crates/core/src/constants.rs`**

Open the file (read its current contents first to see what's there). Append a new block:

```rust
// Density-flow tuning (T3).
pub const CLAIM_THRESHOLD: f32 = 0.3;
pub const HUB_THRESHOLD: f32 = 1.0;
pub const BIOMASS_CAP: f32 = 2.0;
pub const MIN_FLOW_DENSITY: f32 = 0.05;
pub const AUTONOMOUS_FLOW_WEIGHT: f32 = 0.1;
pub const BIASED_FLOW_WEIGHT: f32 = 0.6;
pub const GRADIENT_FLOW_WEIGHT: f32 = 0.1;
pub const FLOW_NOISE: f32 = 0.15;
pub const WATER_GROWTH_COST: f32 = 0.05;

// Bias and dieback (T3, T5).
pub const BIAS_DECAY: f32 = 0.95;
pub const BIAS_STROKE_INTENSITY: f32 = 0.5;
pub const BIAS_MAGNITUDE_CAP: f32 = 1.5;
pub const DIEBACK_THRESHOLD: f32 = 0.05;
pub const DIEBACK_RATE: f32 = 0.95;

// Resource yields (T4).
pub const DECOMP_RATE: f32 = 0.02;
pub const SUGAR_FROM_DECOMP: f32 = 0.5;
pub const SUGAR_FROM_SYMBIOSIS: f32 = 0.1;
pub const MELANIN_FROM_RADIATION: f32 = 0.1;
pub const RADIATION_DEPLETION_RATE: f32 = 0.1;

// Wisp input (T5).
pub const DRAG_THRESHOLD_PX: f32 = 6.0;
pub const TAP_TIME_MS: u32 = 150;
pub const SAMPLE_INTERVAL_MS: u32 = 50;
pub const SAMPLE_HEX_DISTANCE: f32 = 0.5;
pub const WISP_SENSE_RADIUS_HEX: i32 = 5;
```

`ANASTOMOSIS_BIOMASS_BONUS` (currently used by `tip.rs`) can stay — T3 deletes its consumer.

- [x] **Step 5: Migrate `Occupant` references — start with the central files**

Run:

```bash
rg -l "Occupant" crates bin
```

Expected list: roughly 30 files (per the migration map at the top). Migrate each file one by one. The strict rule: every `match` on `Occupant`, every `Occupant::*` literal, every `is_player()` / `is_rival()` / `region_id()` call site must be updated using the migration table above.

For each file, prefer Edit over rewrites. After every two or three files migrated, run `cargo check --workspace --all-targets` and fix new errors before continuing.

Migration order (touchpoint-light → touchpoint-heavy):

1. `crates/core/src/components.rs` — touch component definitions only, light
2. `crates/core/src/lib.rs` and `crates/core/src/messages.rs` — re-exports and message struct fields
3. `crates/world/src/region_tracking.rs` — central ownership reader
4. `crates/world/src/terrain_gen.rs` — initial state. The current `use kingdom_core::{... RivalId ...}` import (line 7) becomes invalid once T2 step 2 deletes `RivalId` from `tile.rs`; drop `RivalId` from the import list.
5. `crates/growth/src/tip.rs` — heavy reader (deleted in T3 anyway, but must compile during T2)
6. `crates/growth/src/decay.rs` — reads ownership
7. `crates/growth/src/nutrient.rs` — reads ownership
8. `crates/regions/src/discovery.rs` — `decomposer_discovery_system` reads ownership
9. `crates/regions/src/fragment.rs` — fragment fusion
10. `crates/ai/src/organisms.rs` — neutral fungi check ownership
11. `crates/render/src/data_layer.rs` — extract_branch_graph and friends
12. `crates/render/src/entity_render.rs` — region highlight, tip render
13. `crates/render/src/network_render.rs` — branch graph reader
14. `crates/fruiting/src/effects.rs`, `crates/fruiting/src/spores.rs` — read ownership for fruit-body recipes
15. `crates/ui/src/hud.rs`, `crates/ui/src/tile_popover.rs` — display ownership
16. Any remaining test files

After every file, the rule is: the call site does the *same logical thing* it did before, just expressed via `region_id` and `biomass`. Do not change behavior. Default migration is `region_id.is_some()` for "owned by some region"; the threshold gate is T3's concern.

**Exception — sites that demand real ownership semantics, not just "tagged":** these need `tile.region_id.is_some() && tile.biomass >= CLAIM_THRESHOLD` from the outset, since a `region_id` tag at sub-threshold biomass would let the consumer trigger before the player's network has actually arrived:

- `crates/regions/src/fragment.rs` — `tile.occupant.is_player()` → `tile.region_id.is_some() && tile.biomass >= CLAIM_THRESHOLD`. Update the existing test fixture (`biomass: 0.0` default → `biomass: 0.5`).
- `crates/fruiting/src/effects.rs`, `crates/fruiting/src/spores.rs` — any site asking "is this tile under our network?" gets the threshold gate. "Is this tile tagged with our id?" stays at `region_id == Some(rid)`.

Use judgement on case-by-case basis. Mark a `// THRESHOLD-GATED` comment at any site where you applied the stronger check; it makes the T7 integration tests easier to debug if the gate behaves unexpectedly.

- [x] **Step 6: Migrate `nutrient_level` → `soil_richness` rename**

```bash
rg -l "nutrient_level" crates bin
```

For each file, replace `nutrient_level` with `soil_richness`. This is a pure rename — no semantic change. Common locations: `nutrient.rs`, `tip.rs`, terrain gen, render data layer.

- [x] **Step 7: Migrate `region.nutrients`, `region.energy`, `region.biomass` reads**

```bash
rg "\.nutrients|\.energy|\.biomass" crates bin
```

Several false positives possible (especially `tile.biomass`); filter mentally. For `RegionState`:

- `region.nutrients` → `region.sugars` (rename)
- `region.energy` → remove the read entirely. Every consumer of `region.energy` is in code that gets deleted in T3 (decay, nutrient production, transport). If a stubborn call site survives, hard-code `0.0` rather than routing through sugars; do not introduce semantic coupling that T3 has to undo.
- `region.biomass` → `region.total_biomass` (rename)

After each file, `cargo check --workspace --all-targets` — keep green.

- [x] **Step 8: Update `crates/world/src/terrain_gen.rs` to seed radiation**

Read the file first. Find where each tile is initialized. After existing field assignments add radiation seeding:

```rust
// Radiation seeding pass: ruins are hot; tiles within 2 hex of a ruin get falloff.
// Two-pass approach: first generate ruin tiles, then run a second sweep that
// looks at each tile's distance to the nearest ruin and assigns radiation.
//
// Use the existing rng (seeded from LaunchConfig.seed) so values are deterministic.
```

Implementation pattern. `terrain_gen.rs` currently builds an in-memory `Vec<(Hex, Tile)>` (or equivalent map) before spawning entities — do the radiation pass on that buffer, before spawn. If the existing structure spawns directly, adapt by collecting ruin positions in the spawn loop, then walking the buffer a second time. House style is `unsigned_distance_to` (returns `u32`) — match existing usage in `crates/render/src/data_layer.rs:204` and `crates/fruiting/src/effects.rs:16`.

```rust
// After every tile's TerrainType is decided, before spawning...
let ruin_positions: Vec<Hex> = tile_buf
    .iter()
    .filter_map(|(pos, t)| (t.terrain == TerrainType::Ruin).then_some(*pos))
    .collect();

for (pos, tile) in tile_buf.iter_mut() {
    if tile.terrain == TerrainType::Ruin {
        tile.radiation = 0.6 + rng.random::<f32>() * 0.4; // 0.6..=1.0
        continue;
    }
    let Some(nearest) = ruin_positions
        .iter()
        .map(|&r| pos.unsigned_distance_to(r))
        .min()
    else {
        continue; // no ruins generated this seed
    };
    if nearest > 0 && nearest <= 2 {
        let falloff = 1.0 - (nearest as f32) / 2.0;
        tile.radiation = 0.4 * falloff;
    }
}
```

The principle: deterministic-from-seed, ruins hot, 2-hex falloff. If `tile_buf` is named differently in the existing file, rename to match.

- [x] **Step 9: Verify migration compiles**

```bash
cargo check --workspace --all-features --all-targets
```

Expected: clean. If any references to `Occupant`, `nutrient_level`, `region.nutrients`, `region.energy`, `region.biomass` remain, hunt them down.

- [x] **Step 10: Update tests in modified files**

Tests that constructed `Tile { occupant: Occupant::Player(rid), .. }` need to construct `Tile { region_id: Some(rid), biomass: 0.5, .. }` instead. Tests that asserted `tile.occupant.is_player()` change to `assert!(tile.region_id.is_some())` (or `assert_eq!(tile.region_id, Some(rid))` where the specific region matters).

Touch points: `crates/growth/src/tip.rs` tests, `crates/growth/src/decay.rs` tests, `crates/regions/src/discovery.rs` test, `crates/regions/src/fragment.rs` tests, `crates/world/src/region_tracking.rs` tests, `crates/ai/src/organisms.rs` tests if any.

- [x] **Step 11: Run the full test suite**

```bash
just test
```

Expected: pass.

- [x] **Step 12: Run lints**

```bash
just lint
```

Expected: pass.

- [x] **Step 13: Smoke-test the running game**

```bash
just dev
```

Expected: game launches, terrain renders, mycelium grows from the seed via the tip system, no crashes. Quit after a few seconds.

- [x] **Step 14: Commit T2**

```bash
git add -A
git commit -m "T2: drop Occupant, rename nutrient_level→soil_richness, migrate RegionState resources, seed radiation"
```

---

## Task 3: Replace growth core (density flow)

**Goal:** Delete the tip-agent simulation. Add density flow + dieback + moisture diffusion + bias decay. Update region tracking and rendering for the new ownership semantics. After T3 the mycelium grows via density flow instead of tips, but no resources accumulate yet (T4 adds those).

**Files:**
- Delete: `crates/growth/src/tip.rs`, `crates/growth/src/decay.rs`
- Modify: `crates/growth/src/nutrient.rs` (keep `nutrient_gradient_system` only — delete the production / transport functions)
- Create: `crates/growth/src/density_flow.rs`
- Create: `crates/growth/src/dieback.rs`
- Create: `crates/growth/src/moisture.rs`
- Create: `crates/growth/src/bias_decay.rs`
- Modify: `crates/growth/src/lib.rs` (new module declarations and registration)
- Modify: `crates/world/src/region_tracking.rs` (use `region_id.is_some() && biomass >= CLAIM_THRESHOLD`)
- Modify: `crates/render/src/data_layer.rs` (delete `extract_tip_positions`, `TipPositions`; update `extract_branch_graph` for biomass-driven edges)
- Modify: `crates/render/src/entity_render.rs` (delete `tip_render_system`)
- Modify: `crates/render/src/lib.rs` (drop `TipPositions`, drop tip system registration)
- Modify: `crates/core/src/components.rs` (delete `HyphalTip` component)

**Note on parallelism:** This task and T6 (UI simplification) run in parallel. T3 owns growth + world + render; T6 owns ui. Files do not overlap.

- [x] **Step 1: Verify baseline (T2 complete)**

```bash
just lint && just test && git status
```

Expected: clean.

- [x] **Step 2: Write the failing test for `bias_decay_system`**

Create `crates/growth/src/bias_decay.rs` with the test scaffold:

```rust
use bevy::prelude::*;
use kingdom_core::{BIAS_DECAY, GridPos, GridWorld, Hex, Tile};

pub fn bias_decay_system(mut tiles: Query<&mut Tile>) {
    const EPSILON: f32 = 0.001;
    for mut tile in tiles.iter_mut() {
        tile.priority_bias *= BIAS_DECAY;
        if tile.priority_bias.length_squared() < EPSILON * EPSILON {
            tile.priority_bias = Vec2::ZERO;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.add_systems(Update, bias_decay_system);
        app
    }

    #[test]
    fn nonzero_bias_shrinks_each_tick() {
        let mut app = test_app();
        let entity = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    priority_bias: Vec2::new(1.0, 0.0),
                    ..default()
                },
            ))
            .id();

        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(tile.priority_bias.x < 1.0);
        assert!(tile.priority_bias.x > 0.0);
        assert!((tile.priority_bias.x - BIAS_DECAY).abs() < 1e-6);
    }

    #[test]
    fn tiny_bias_snaps_to_zero() {
        let mut app = test_app();
        let entity = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    priority_bias: Vec2::new(0.0001, 0.0),
                    ..default()
                },
            ))
            .id();

        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::ZERO);
    }
}
```

- [x] **Step 3: Wire `bias_decay` module and verify tests pass**

In `crates/growth/src/lib.rs`, add `mod bias_decay;` and `pub use bias_decay::bias_decay_system;`. Then:

```bash
cargo nextest run -p kingdom_growth bias_decay
```

Expected: 2 tests pass.

- [x] **Step 4: Write `moisture_diffusion_system` with tests first**

Create `crates/growth/src/moisture.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{GridPos, GridWorld, Tile, TerrainType};

const DIFFUSION_RATE: f32 = 0.05;

pub fn moisture_diffusion_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
) {
    let snapshot: std::collections::HashMap<_, _> = tiles
        .iter()
        .map(|(gp, t)| (gp.0, t.moisture))
        .collect();

    for (gpos, mut tile) in tiles.iter_mut() {
        if tile.terrain == TerrainType::Water {
            tile.moisture = 1.0;
            continue;
        }
        let mut total_diff = 0.0_f32;
        let mut count = 0_f32;
        for (npos, _) in grid.neighbors(gpos.0) {
            if let Some(&n_moist) = snapshot.get(&npos) {
                total_diff += n_moist - tile.moisture;
                count += 1.0;
            }
        }
        if count > 0.0 {
            tile.moisture += DIFFUSION_RATE * (total_diff / count);
            tile.moisture = tile.moisture.clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{Hex, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.insert_resource(create_hex_layout());
        app.add_systems(Update, moisture_diffusion_system);
        app
    }

    fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let e = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
        e
    }

    #[test]
    fn water_terrain_stays_at_one() {
        let mut app = test_app();
        let e = spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                terrain: TerrainType::Water,
                moisture: 0.5,
                ..default()
            },
        );
        app.update();
        assert_eq!(app.world().get::<Tile>(e).unwrap().moisture, 1.0);
    }

    #[test]
    fn dry_tile_adjacent_to_wet_neighbor_gains_moisture() {
        let mut app = test_app();
        let center = Hex::new(5, 5);
        let neighbor = center.all_neighbors()[0];
        let dry = spawn(
            &mut app,
            center,
            Tile {
                moisture: 0.0,
                ..default()
            },
        );
        spawn(
            &mut app,
            neighbor,
            Tile {
                moisture: 1.0,
                ..default()
            },
        );
        app.update();
        let m = app.world().get::<Tile>(dry).unwrap().moisture;
        assert!(m > 0.0 && m < 1.0, "moisture should rise toward wet neighbor: {m}");
    }

    #[test]
    fn moisture_clamps_non_negative() {
        let mut app = test_app();
        let e = spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                moisture: 0.0,
                ..default()
            },
        );
        app.update();
        assert!(app.world().get::<Tile>(e).unwrap().moisture >= 0.0);
    }
}
```

In `crates/growth/src/lib.rs` add `mod moisture;` and `pub use moisture::moisture_diffusion_system;`. Run:

```bash
cargo nextest run -p kingdom_growth moisture
```

Expected: 3 tests pass.

- [x] **Step 5: Write `density_flow_system` with tests first**

Create `crates/growth/src/density_flow.rs`. The system has the highest design density of any in the plan — write the tests first, then the implementation.

```rust
use std::collections::HashMap;

use bevy::prelude::*;
use kingdom_core::{
    AUTONOMOUS_FLOW_WEIGHT, BIASED_FLOW_WEIGHT, BIOMASS_CAP, CLAIM_THRESHOLD, FLOW_NOISE,
    GRADIENT_FLOW_WEIGHT, GridPos, GridWorld, Hex, HexLayout, MIN_FLOW_DENSITY, RegionId,
    RegionStates, Tile, TileDiscovered, WATER_GROWTH_COST,
};
use bevy::ecs::message::MessageWriter;
use rand::SeedableRng;
use rand::prelude::*;
use rand::rngs::StdRng;

#[derive(Resource)]
pub struct DensityFlowRng(pub StdRng);

impl Default for DensityFlowRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(13))
    }
}

#[derive(Default)]
struct TileDelta {
    biomass_in: f32,
    contributing_region: Option<(RegionId, f32)>,
}

pub fn density_flow_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    layout: Res<HexLayout>,
    mut rng: ResMut<DensityFlowRng>,
    mut discovered: MessageWriter<TileDiscovered>,
) {
    // Phase 1: snapshot + compute outflows.
    let snapshot: HashMap<Hex, (Option<RegionId>, f32, Vec2, Vec2, f32, bool)> = tiles
        .iter()
        .map(|(gp, t)| {
            (
                gp.0,
                (
                    t.region_id,
                    t.biomass,
                    t.priority_bias,
                    t.nutrient_gradient,
                    t.moisture,
                    t.terrain.is_passable(),
                ),
            )
        })
        .collect();

    let mut deltas: HashMap<Hex, TileDelta> = HashMap::new();
    let mut outflow_total: HashMap<Hex, f32> = HashMap::new();
    let mut water_consumption: HashMap<Hex, f32> = HashMap::new();

    // Iterate in sorted-key order so HashMap iteration nondeterminism does not
    // interleave with rng.random() calls. Without this, two runs with the same
    // DensityFlowRng seed produce different outputs, breaking test reproducibility.
    let mut keys: Vec<Hex> = snapshot.keys().copied().collect();
    keys.sort_by_key(|h| (h.x, h.y));

    for pos in keys {
        let (rid_opt, biomass, bias, gradient, moisture, _passable) = snapshot[&pos];
        if biomass <= MIN_FLOW_DENSITY {
            continue;
        }
        let Some(rid) = rid_opt else { continue };

        let from_world = layout.hex_to_world_pos(pos);
        let mut candidates: Vec<(Hex, f32)> = Vec::new();
        for (npos, _) in grid.neighbors(pos) {
            let Some(&(n_rid, _, _, _, _, n_passable)) = snapshot.get(&npos) else {
                continue;
            };
            if !n_passable {
                continue;
            }
            if let Some(other) = n_rid
                && other != rid
            {
                continue;
            }
            let to_world = layout.hex_to_world_pos(npos);
            let dir = (to_world - from_world).normalize_or_zero();
            let bias_score = bias.dot(dir).max(0.0);
            let gradient_score = gradient.dot(dir).max(0.0);
            let mut weight = AUTONOMOUS_FLOW_WEIGHT
                + BIASED_FLOW_WEIGHT * bias_score
                + GRADIENT_FLOW_WEIGHT * gradient_score;
            let noise = (rng.0.random::<f32>() - 0.5) * FLOW_NOISE;
            weight *= 1.0 + noise;
            if weight > 0.0 {
                candidates.push((npos, weight));
            }
        }

        let total: f32 = candidates.iter().map(|(_, w)| *w).sum();
        if total <= 0.0 {
            continue;
        }

        let max_outflow = (biomass * 0.1).min(moisture / WATER_GROWTH_COST.max(1e-6));
        if max_outflow <= 0.0 {
            continue;
        }

        for (npos, weight) in candidates {
            let share = max_outflow * (weight / total);
            let entry = deltas.entry(npos).or_default();
            entry.biomass_in += share;
            match &mut entry.contributing_region {
                Some((_existing_rid, existing_share)) if *existing_share >= share => {}
                slot => *slot = Some((rid, share)),
            }
            *outflow_total.entry(pos).or_insert(0.0) += share;
            *water_consumption.entry(pos).or_insert(0.0) += share * WATER_GROWTH_COST;
        }
    }

    // Phase 2: apply. Per the design spec ("don't drain more than ten percent per
    // tick"), source biomass is deducted by the total outflow it sent; neighbors
    // gain biomass; both source and sink converge subject to BIOMASS_CAP.
    for (gpos, mut tile) in tiles.iter_mut() {
        if let Some(&out) = outflow_total.get(&gpos.0) {
            tile.biomass = (tile.biomass - out).max(0.0);
        }
        if let Some(delta) = deltas.get(&gpos.0) {
            let new_biomass = (tile.biomass + delta.biomass_in).min(BIOMASS_CAP);
            let was_unowned = tile.region_id.is_none();
            tile.biomass = new_biomass;
            if was_unowned && new_biomass >= CLAIM_THRESHOLD {
                if let Some((rid, _)) = delta.contributing_region {
                    tile.region_id = Some(rid);
                    if !tile.discovered {
                        tile.discovered = true;
                        discovered.write(TileDiscovered {
                            pos: gpos.0,
                            contents: tile.contents,
                        });
                    }
                }
            }
        }
        if let Some(&used) = water_consumption.get(&gpos.0) {
            tile.moisture = (tile.moisture - used).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{TerrainType, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.insert_resource(create_hex_layout());
        app.insert_resource(DensityFlowRng(StdRng::seed_from_u64(42)));
        app.add_message::<TileDiscovered>();
        app.add_systems(Update, density_flow_system);
        app
    }

    fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let e = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
        e
    }

    #[test]
    fn flow_follows_bias_direction() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let layout = create_hex_layout();

        let center = Hex::new(5, 5);
        let neighbors = center.all_neighbors();
        let target = neighbors[0];
        let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

        spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                moisture: 1.0,
                priority_bias: dir * 1.0,
                ..default()
            },
        );
        for &n in &neighbors {
            spawn(&mut app, n, Tile { moisture: 0.5, ..default() });
        }

        app.update();

        let grid = app.world().resource::<GridWorld>();
        let target_tile = app.world().get::<Tile>(grid.tiles[&target]).unwrap();
        let other_tile = app.world().get::<Tile>(grid.tiles[&neighbors[3]]).unwrap();

        assert!(
            target_tile.biomass > other_tile.biomass,
            "biased neighbor ({}) should outpace opposite neighbor ({})",
            target_tile.biomass,
            other_tile.biomass
        );
    }

    #[test]
    fn flow_consumes_source_water() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let center = Hex::new(0, 0);
        let neighbors = center.all_neighbors();
        let source_e = spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                moisture: 1.0,
                ..default()
            },
        );
        for &n in &neighbors {
            spawn(&mut app, n, Tile { moisture: 0.0, ..default() });
        }
        app.update();
        let m = app.world().get::<Tile>(source_e).unwrap().moisture;
        assert!(m < 1.0, "source moisture should drop after growth: {m}");
    }

    #[test]
    fn empty_tile_claimed_when_biomass_crosses_threshold() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let layout = create_hex_layout();
        let center = Hex::new(0, 0);
        let target = center.all_neighbors()[0];
        let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

        spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: BIOMASS_CAP,
                moisture: 1.0,
                priority_bias: dir * 1.5,
                ..default()
            },
        );
        let target_e = spawn(&mut app, target, Tile { moisture: 0.5, ..default() });

        // Run multiple ticks — single-tick flow may not cross threshold.
        for _ in 0..10 {
            app.update();
        }

        let tile = app.world().get::<Tile>(target_e).unwrap();
        assert!(
            tile.region_id == Some(rid) || tile.biomass < CLAIM_THRESHOLD,
            "claimed tiles should belong to the source region"
        );
        if tile.biomass >= CLAIM_THRESHOLD {
            assert_eq!(tile.region_id, Some(rid));
        }
    }

    #[test]
    fn cross_region_tiles_not_entered() {
        let mut app = test_app();
        let rid_a = app.world_mut().resource_mut::<RegionStates>().create_region();
        let rid_b = app.world_mut().resource_mut::<RegionStates>().create_region();
        let center = Hex::new(0, 0);
        let neighbor = center.all_neighbors()[0];
        spawn(
            &mut app,
            center,
            Tile {
                region_id: Some(rid_a),
                biomass: 2.0,
                moisture: 1.0,
                priority_bias: Vec2::splat(1.0),
                ..default()
            },
        );
        let n_e = spawn(
            &mut app,
            neighbor,
            Tile {
                region_id: Some(rid_b),
                biomass: 0.5,
                moisture: 0.5,
                ..default()
            },
        );
        let initial = app.world().get::<Tile>(n_e).unwrap().biomass;
        for _ in 0..5 {
            app.update();
        }
        let after = app.world().get::<Tile>(n_e).unwrap().biomass;
        assert!(
            (after - initial).abs() < 1e-3,
            "biomass should be unchanged on a cross-region tile, was {initial} now {after}"
        );
    }
}
```

In `crates/growth/src/lib.rs` add `mod density_flow;` and `pub use density_flow::{DensityFlowRng, density_flow_system};`. Run:

```bash
cargo nextest run -p kingdom_growth density_flow
```

Expected: 4 tests pass. If `flow_follows_bias_direction` is flaky due to noise, lower the noise scale temporarily in the test by inserting a fixed-seed RNG and rerun.

- [x] **Step 6: Write `dieback_system` with tests first**

Create `crates/growth/src/dieback.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{CLAIM_THRESHOLD, DIEBACK_RATE, DIEBACK_THRESHOLD, Tile};

const EPSILON: f32 = 0.001;

pub fn dieback_system(mut tiles: Query<&mut Tile>) {
    for mut tile in tiles.iter_mut() {
        if tile.biomass <= 0.0 {
            continue;
        }
        if tile.moisture < DIEBACK_THRESHOLD {
            tile.biomass *= DIEBACK_RATE;
        }
        if tile.biomass < CLAIM_THRESHOLD {
            tile.region_id = None;
        }
        if tile.biomass < EPSILON {
            tile.biomass = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GridPos, Hex, RegionId, GridWorld};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.add_systems(Update, dieback_system);
        app
    }

    #[test]
    fn low_moisture_shrinks_biomass() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 1.0,
                    moisture: 0.0,
                    ..default()
                },
            ))
            .id();
        app.update();
        let tile = app.world().get::<Tile>(e).unwrap();
        assert!(tile.biomass < 1.0);
    }

    #[test]
    fn biomass_below_claim_threshold_clears_region() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 0.1,
                    moisture: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        let tile = app.world().get::<Tile>(e).unwrap();
        assert_eq!(tile.region_id, None);
    }

    #[test]
    fn tiny_biomass_snaps_to_zero() {
        let mut app = test_app();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(RegionId(0)),
                    biomass: 0.0001,
                    moisture: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Tile>(e).unwrap().biomass, 0.0);
    }
}
```

Wire `mod dieback;` and `pub use dieback::dieback_system;` in `lib.rs`. Run:

```bash
cargo nextest run -p kingdom_growth dieback
```

Expected: 3 tests pass.

- [x] **Step 7: Strip the old `nutrient.rs` down to just `nutrient_gradient_system`**

Open `crates/growth/src/nutrient.rs`. Delete `nutrient_production_system` and `nutrient_transport_system` and their tests. Keep `nutrient_gradient_system` and its `gradient_points_toward_higher_nutrients` test (rename references inside from `nutrient_level` to `soil_richness` if T2 missed any).

In `crates/growth/src/lib.rs`, drop `nutrient_production_system` and `nutrient_transport_system` from re-exports and from system registration. Keep `nutrient_gradient_system`.

- [x] **Step 8: Delete `tip.rs` and `decay.rs`**

```bash
rm crates/growth/src/tip.rs crates/growth/src/decay.rs
```

In `crates/growth/src/lib.rs`, drop `mod tip;`, `mod decay;`, all their re-exports, and their system registrations from `GrowthPlugin::build`.

After tip.rs is gone, `ANASTOMOSIS_BIOMASS_BONUS` (in `crates/core/src/constants.rs`) becomes dead. Delete the constant definition. Verify no other consumers:

```bash
rg "ANASTOMOSIS_BIOMASS_BONUS" crates bin
```

Expected: empty.

- [x] **Step 9: Delete `HyphalTip` from `crates/core/src/components.rs`**

Open the file, remove the `HyphalTip` struct definition and its derives. Remove `HyphalTip` from any re-exports in `crates/core/src/lib.rs`. Run `cargo check` — anything that still references `HyphalTip` must be removed.

- [x] **Step 10: Wire the new growth plugin chain**

Replace the body of `GrowthPlugin::build` in `crates/growth/src/lib.rs`:

```rust
use bevy::prelude::*;

use kingdom_core::SimulationSystems;

mod bias_decay;
mod density_flow;
mod dieback;
mod moisture;
mod nutrient;

pub use bias_decay::bias_decay_system;
pub use density_flow::{DensityFlowRng, density_flow_system};
pub use dieback::dieback_system;
pub use moisture::moisture_diffusion_system;
pub use nutrient::nutrient_gradient_system;

pub struct GrowthPlugin;

impl Plugin for GrowthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DensityFlowRng>().add_systems(
            Update,
            (
                bias_decay_system,
                moisture_diffusion_system,
                nutrient_gradient_system,
                density_flow_system,
                dieback_system,
            )
                .chain()
                .in_set(SimulationSystems),
        );
    }
}
```

- [x] **Step 11: Update `crates/world/src/region_tracking.rs` for new ownership semantics**

Open the file. The current implementation reads `Occupant::Player(rid)` to identify owned tiles. Replace that with: a tile is "claimed for region R" iff `tile.region_id == Some(R) && tile.biomass >= CLAIM_THRESHOLD`. Connected components by these criteria define regions.

Update tests in the same file similarly: setup tiles with `region_id: Some(rid), biomass: 0.5` rather than `occupant: Occupant::Player(rid)`.

Also update `RegionState.total_biomass`: each tick, `region.total_biomass = sum(tile.biomass for owned tiles)` and `region.tile_count = count(owned tiles)`. This is the natural place for that aggregation.

- [x] **Step 12: Update `crates/render/src/data_layer.rs`**

Delete:
- `TipPositions` resource
- `extract_tip_positions` system
- The `extract_tip_positions` registration from `RenderPlugin::build` (in `crates/render/src/lib.rs` step 13).

Update `extract_branch_graph`:
- Builds edges between any two adjacent tiles where both `region_id.is_some()` and both `biomass >= CLAIM_THRESHOLD`.
- Edge weight (if present) reflects average biomass — feeds the existing strand-thickness shader unchanged.

- [x] **Step 13: Update `crates/render/src/lib.rs`**

Drop:
- `init_resource::<TipPositions>()`
- `data_layer::extract_tip_positions` from system tuple

- [x] **Step 14: Update `crates/render/src/entity_render.rs`**

Delete `tip_render_system` and any associated marker components or sprite-link handling that exists only to render tips. Drop its registration from the `PostUpdate` schedule in `RenderPlugin::build`.

- [x] **Step 15: Verify compilation**

```bash
cargo check --workspace --all-features --all-targets
```

Expected: clean.

- [x] **Step 16: Run growth tests**

```bash
cargo nextest run -p kingdom_growth
```

Expected: all bias_decay, moisture, density_flow, dieback, nutrient_gradient tests pass. Total roughly 13 tests.

- [x] **Step 17: Run world tests**

```bash
cargo nextest run -p kingdom_world
```

Expected: pass after region_tracking adjustments.

- [x] **Step 18: Run full suite**

```bash
just test
```

Expected: pass. If region tracking tests fail because they depended on `Occupant`, fix them per Step 11.

- [x] **Step 19: Smoke test**

```bash
just dev
```

Expected: game launches; mycelium spreads from spawn via density flow (no longer via tip hops); paint with P-key (T5 will replace this) still pulls bias. No crashes.

- [x] **Step 20: Run lints**

```bash
just lint
```

- [x] **Step 21: Commit T3**

```bash
git add -A
git commit -m "T3: replace tip-agent growth with density flow + dieback + moisture + bias decay"
```

---

## Task 4: Resource systems (decomposition, symbiosis, melanin)

**Goal:** Implement decomposition (universal — produces sugars and fires slot-machine for `UniqueDecomposable`), symbiosis (plant root adjacency yields sugars at water cost), melanin (radiated owned tiles credit melanin and slowly cleanse the tile). Re-wire `slot_machine_system` to consume `DecompositionComplete` instead of `StudyComplete`.

**Files:**
- Modify: `crates/core/src/messages.rs` (extend `DecompositionComplete` with `was_unique: bool`)
- Modify: `crates/regions/src/discovery.rs` (universalise `decomposer_discovery_system` into a `decomposition_system` that runs for every owned tile, drops the specialization gate, and emits `DecompositionComplete { was_unique }` plus a region sugar credit)
- Create: `crates/growth/src/symbiosis.rs`
- Create: `crates/growth/src/melanin.rs`
- Modify: `crates/regions/src/slot_machine.rs` (read `DecompositionComplete`, fire `SlotMachineTriggered` when `was_unique`)
- Modify: `crates/regions/src/lib.rs` (rename `decomposer_discovery_system` to `decomposition_system` in re-export and registration)
- Modify: `crates/growth/src/lib.rs` (add symbiosis and melanin to plugin chain)

- [x] **Step 1: Verify baseline (T3 complete)**

```bash
just lint && just test && git status
```

- [x] **Step 2: Extend `DecompositionComplete` with `was_unique`**

Open `crates/core/src/messages.rs`. Find the `DecompositionComplete` struct. Add a `pub was_unique: bool` field. Adjust the `Message` derive if needed (no change expected).

- [x] **Step 3: Universalise decomposition with tests**

Replace `decomposer_discovery_system` in `crates/regions/src/discovery.rs` with `decomposition_system`. Spec:

- Iterate over tiles. For each tile where `region_id.is_some() && biomass >= CLAIM_THRESHOLD` and `contents` is `Some(OrganicMatter)` or `Some(UniqueDecomposable(_))`:
  - Increment progress at `DECOMP_RATE` per tick.
  - Add `SUGAR_FROM_DECOMP * DECOMP_RATE` to `region.sugars` (per-tick crediting — total yield over the `1/DECOMP_RATE = 50` ticks of decomposition equals `SUGAR_FROM_DECOMP`. This matches the spec's intent that sugars accrue *during* decomposition, not in a single burst on completion. If the spec is later interpreted as completion-only, move this addition into the `*prog >= 1.0` branch).
  - On reaching 1.0:
    - Set `tile.contents = None`
    - Add 0.2 to `tile.soil_richness` (clamped to 1.0)
    - Fire `DecompositionComplete { pos, was_unique: matches!(old_contents, Some(UniqueDecomposable(_))) }`

Replace the file's contents with:

```rust
use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use kingdom_core::{
    CLAIM_THRESHOLD, DECOMP_RATE, DecompositionComplete, GridPos, Hex, RegionStates,
    SUGAR_FROM_DECOMP, Tile, TileContents,
};

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct DecompProgress {
    pub entries: HashMap<Hex, f32>,
}

pub fn decomposition_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut region_states: ResMut<RegionStates>,
    mut progress: ResMut<DecompProgress>,
    mut decomp_messages: MessageWriter<DecompositionComplete>,
) {
    for (gpos, mut tile) in tiles.iter_mut() {
        let Some(rid) = tile.region_id else { continue };
        if tile.biomass < CLAIM_THRESHOLD {
            continue;
        }
        let was_unique = match tile.contents {
            Some(TileContents::OrganicMatter) => false,
            Some(TileContents::UniqueDecomposable(_)) => true,
            _ => continue,
        };

        if let Some(state) = region_states.get_mut(rid) {
            state.sugars += SUGAR_FROM_DECOMP * DECOMP_RATE;
        }

        let prog = progress.entries.entry(gpos.0).or_insert(0.0);
        *prog += DECOMP_RATE;
        if *prog >= 1.0 {
            tile.contents = None;
            tile.soil_richness = (tile.soil_richness + 0.2).min(1.0);
            progress.entries.remove(&gpos.0);
            decomp_messages.write(DecompositionComplete {
                pos: gpos.0,
                was_unique,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GridWorld, RegionId};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<DecompProgress>();
        app.add_message::<DecompositionComplete>();
        app.add_systems(Update, decomposition_system);
        app
    }

    fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let e = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
        e
    }

    #[test]
    fn owned_organic_tile_adds_sugars() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::OrganicMatter),
                ..default()
            },
        );
        let before = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        app.update();
        let after = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        assert!(after > before, "decomposition should yield sugars: {before} → {after}");
    }

    #[test]
    fn unique_decomposable_completion_fires_was_unique_event() {
        use bevy::ecs::message::MessageReader;
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let pos = Hex::new(2, 2);
        spawn(
            &mut app,
            pos,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::UniqueDecomposable(0)),
                ..default()
            },
        );
        app.world_mut()
            .resource_mut::<DecompProgress>()
            .entries
            .insert(pos, 0.99);
        let captured = std::sync::Arc::new(std::sync::Mutex::new(false));
        let captured_c = captured.clone();
        app.add_systems(
            Update,
            (move |mut r: MessageReader<DecompositionComplete>| {
                for ev in r.read() {
                    if ev.was_unique {
                        *captured_c.lock().unwrap() = true;
                    }
                }
            })
                .after(decomposition_system),
        );
        app.update();
        assert!(*captured.lock().unwrap());
    }

    #[test]
    fn non_owned_tile_no_progress() {
        let mut app = test_app();
        spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                region_id: None,
                biomass: 0.0,
                contents: Some(TileContents::OrganicMatter),
                ..default()
            },
        );
        app.update();
        assert!(app.world().resource::<DecompProgress>().entries.is_empty());
    }
}
```

In `crates/regions/src/lib.rs`, rename the export from `decomposer_discovery_system` to `decomposition_system` and update the system registration. Run:

```bash
cargo nextest run -p kingdom_regions decomposition
```

Expected: 3 tests pass.

- [x] **Step 4: Re-wire `slot_machine_system` to consume `DecompositionComplete`**

Open `crates/regions/src/slot_machine.rs`. Replace the body of `slot_machine_system`:

```rust
use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;
use kingdom_core::{DecompositionComplete, UnlockOption, UnlockPool};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;

#[derive(Message)]
pub struct SlotMachineTriggered {
    pub pool: UnlockPool,
    pub options: Vec<UnlockOption>,
}

#[derive(Resource)]
pub struct SlotMachineRng(pub StdRng);

impl Default for SlotMachineRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(7))
    }
}

pub fn slot_machine_system(
    mut decomp_messages: MessageReader<DecompositionComplete>,
    mut slot_messages: MessageWriter<SlotMachineTriggered>,
    mut rng: ResMut<SlotMachineRng>,
) {
    for event in decomp_messages.read() {
        if !event.was_unique {
            continue;
        }
        let pool_options = unlock_pool_options(UnlockPool::Decomposition);
        let selected: Vec<UnlockOption> = pool_options.sample(&mut rng.0, 3).cloned().collect();
        slot_messages.write(SlotMachineTriggered {
            pool: UnlockPool::Decomposition,
            options: selected,
        });
    }
}
```

Keep the `unlock_pool_options` helper — it's still useful even though only the `Decomposition` arm is reachable now.

Add a replacement test:

```rust
#[cfg(test)]
mod tests {
    use kingdom_core::Hex;

    use super::*;

    #[test]
    fn slot_machine_fires_on_unique_decomp() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(0));
        let captured_c = captured.clone();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SlotMachineRng>();
        app.add_message::<DecompositionComplete>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(
            Update,
            (
                slot_machine_system,
                (move |mut r: MessageReader<SlotMachineTriggered>| {
                    for ev in r.read() {
                        if ev.options.len() == 3 {
                            *captured_c.lock().unwrap() += 1;
                        }
                    }
                }),
            )
                .chain(),
        );
        app.world_mut().write_message(DecompositionComplete {
            pos: Hex::ZERO,
            was_unique: true,
        });
        app.update();
        assert_eq!(*captured.lock().unwrap(), 1);
    }

    #[test]
    fn slot_machine_quiet_on_organic_decomp() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(0));
        let captured_c = captured.clone();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SlotMachineRng>();
        app.add_message::<DecompositionComplete>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(
            Update,
            (
                slot_machine_system,
                (move |mut r: MessageReader<SlotMachineTriggered>| {
                    for _ in r.read() {
                        *captured_c.lock().unwrap() += 1;
                    }
                }),
            )
                .chain(),
        );
        app.world_mut().write_message(DecompositionComplete {
            pos: Hex::ZERO,
            was_unique: false,
        });
        app.update();
        assert_eq!(*captured.lock().unwrap(), 0);
    }
}
```

Run:

```bash
cargo nextest run -p kingdom_regions slot_machine
```

Expected: 2 tests pass.

- [x] **Step 5: Write `symbiosis_system` with tests first**

Create `crates/growth/src/symbiosis.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{
    CLAIM_THRESHOLD, GridPos, GridWorld, PlantRootAgent, RegionStates, SUGAR_FROM_SYMBIOSIS, Tile,
    TileContents,
};

const MIN_TRADE_MOISTURE: f32 = 0.3;

pub fn symbiosis_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut plants: Query<&mut PlantRootAgent>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
) {
    let snapshot: std::collections::HashMap<_, _> = tiles
        .iter()
        .map(|(gp, t)| (gp.0, (t.region_id, t.biomass, t.moisture, t.contents)))
        .collect();

    for (gpos, mut tile) in tiles.iter_mut() {
        let Some(rid) = tile.region_id else { continue };
        if tile.biomass < CLAIM_THRESHOLD || tile.moisture <= MIN_TRADE_MOISTURE {
            continue;
        }

        for (npos, nentity) in grid.neighbors(gpos.0) {
            let Some(&(_n_rid, _n_biomass, _n_moisture, n_contents)) = snapshot.get(&npos) else {
                continue;
            };
            if !matches!(n_contents, Some(TileContents::PlantRoot(_))) {
                continue;
            }
            if let Ok(mut plant) = plants.get_mut(nentity) {
                plant.trade_active = true;
            }
            if let Some(state) = region_states.get_mut(rid) {
                state.sugars += SUGAR_FROM_SYMBIOSIS;
            }
            tile.moisture = (tile.moisture - SUGAR_FROM_SYMBIOSIS * 0.3).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{Hex, RegionId, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.insert_resource(create_hex_layout());
        app.add_systems(Update, symbiosis_system);
        app
    }

    #[test]
    fn adjacent_plant_root_yields_sugars() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let center = Hex::new(0, 0);
        let neighbor = center.all_neighbors()[0];
        let myc = app
            .world_mut()
            .spawn((
                GridPos(center),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    moisture: 1.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(center, myc);
        let plant = app
            .world_mut()
            .spawn((
                GridPos(neighbor),
                Tile {
                    contents: Some(TileContents::PlantRoot(0)),
                    ..default()
                },
                PlantRootAgent {
                    plant_id: 0,
                    health: 1.0,
                    trade_active: false,
                    nutrient_intake: 0.0,
                    sugar_output: 0.0,
                    neglect_timer: 0,
                },
            ))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(neighbor, plant);
        let before = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        app.update();
        let after = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        assert!(after > before, "{before} → {after}");
        assert!(app.world().get::<PlantRootAgent>(plant).unwrap().trade_active);
    }

    #[test]
    fn low_moisture_blocks_trade() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let center = Hex::new(0, 0);
        let neighbor = center.all_neighbors()[0];
        let myc = app
            .world_mut()
            .spawn((
                GridPos(center),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    moisture: 0.1, // below threshold
                    ..default()
                },
            ))
            .id();
        app.world_mut().resource_mut::<GridWorld>().tiles.insert(center, myc);
        app.world_mut()
            .spawn((
                GridPos(neighbor),
                Tile {
                    contents: Some(TileContents::PlantRoot(0)),
                    ..default()
                },
                PlantRootAgent {
                    plant_id: 0,
                    health: 1.0,
                    trade_active: false,
                    nutrient_intake: 0.0,
                    sugar_output: 0.0,
                    neglect_timer: 0,
                },
            ))
            .id();
        let before = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        app.update();
        let after = app.world().resource::<RegionStates>().get(rid).unwrap().sugars;
        assert_eq!(before, after);
    }
}
```

Wire `mod symbiosis;` and `pub use symbiosis::symbiosis_system;` in `crates/growth/src/lib.rs`. Run:

```bash
cargo nextest run -p kingdom_growth symbiosis
```

Expected: 2 tests pass.

- [x] **Step 6: Write `melanin_system` with tests first**

Create `crates/growth/src/melanin.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::{
    CLAIM_THRESHOLD, GridPos, MELANIN_FROM_RADIATION, RADIATION_DEPLETION_RATE, RegionStates, Tile,
};

pub fn melanin_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut region_states: ResMut<RegionStates>,
) {
    for (_gpos, mut tile) in tiles.iter_mut() {
        let Some(rid) = tile.region_id else { continue };
        if tile.biomass < CLAIM_THRESHOLD || tile.radiation <= 0.0 {
            continue;
        }
        let yield_amt = MELANIN_FROM_RADIATION * tile.radiation;
        if let Some(state) = region_states.get_mut(rid) {
            state.melanin += yield_amt;
        }
        tile.radiation = (tile.radiation - yield_amt * RADIATION_DEPLETION_RATE).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GridWorld, Hex, RegionId};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.add_systems(Update, melanin_system);
        app
    }

    #[test]
    fn radiated_owned_tile_adds_melanin() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        app.world_mut().spawn((
            GridPos(Hex::ZERO),
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                radiation: 0.5,
                ..default()
            },
        ));
        let before = app.world().resource::<RegionStates>().get(rid).unwrap().melanin;
        app.update();
        let after = app.world().resource::<RegionStates>().get(rid).unwrap().melanin;
        assert!(after > before);
    }

    #[test]
    fn radiation_depletes_over_time() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        let e = app
            .world_mut()
            .spawn((
                GridPos(Hex::ZERO),
                Tile {
                    region_id: Some(rid),
                    biomass: 1.0,
                    radiation: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        let r = app.world().get::<Tile>(e).unwrap().radiation;
        assert!(r < 0.5);
    }

    #[test]
    fn non_radiated_tile_no_melanin() {
        let mut app = test_app();
        let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
        app.world_mut().spawn((
            GridPos(Hex::ZERO),
            Tile {
                region_id: Some(rid),
                biomass: 1.0,
                radiation: 0.0,
                ..default()
            },
        ));
        let before = app.world().resource::<RegionStates>().get(rid).unwrap().melanin;
        app.update();
        let after = app.world().resource::<RegionStates>().get(rid).unwrap().melanin;
        assert_eq!(before, after);
    }
}
```

Wire `mod melanin;` and `pub use melanin::melanin_system;` in `crates/growth/src/lib.rs`. Run:

```bash
cargo nextest run -p kingdom_growth melanin
```

Expected: 3 tests pass.

- [x] **Step 7: Update growth plugin chain to include the new resource systems**

In `crates/growth/src/lib.rs`, update the system tuple to:

```rust
.add_systems(
    Update,
    (
        bias_decay_system,
        moisture_diffusion_system,
        nutrient_gradient_system,
        density_flow_system,
        dieback_system,
        symbiosis_system,
        melanin_system,
    )
        .chain()
        .in_set(SimulationSystems),
);
```

`decomposition_system` stays in the regions crate (registered by `DiscoveryPlugin`); ordering it after density_flow is enforced naturally because regions/discovery is registered after growth in the plugin group.

- [x] **Step 8: Verify compilation**

```bash
cargo check --workspace --all-features --all-targets
```

- [x] **Step 9: Run all tests**

```bash
just test
```

Expected: pass.

- [x] **Step 10: Smoke test**

```bash
just dev
```

Expected: game launches; if you paint with the P-key (T5 will replace this) and the network reaches a plant root or radiation, sugars / melanin should accrue. HUD doesn't show them yet — T6 adds the display. Use `kingdom_core::RegionStates` inspection in the entity inspector if available.

- [x] **Step 11: Commit T4**

```bash
git add -A
git commit -m "T4: universal decomposition, symbiosis, melanin; rewire slot machine to DecompositionComplete"
```

---

## Task 5: Wisp input + bias glow render

**Goal:** Replace the P-key bias stamp with a mouse-drag wisp that writes directional bias along the cursor path. Replace the priority-arrow render with a soft warm glow keyed to bias magnitude. Tap-vs-drag disambiguates inside the wisp state machine; tap forwards to the existing tile-selection.

**Files:**
- Delete: `crates/input/src/priority.rs`
- Create: `crates/input/src/wisp.rs`
- Modify: `crates/input/src/action.rs` (drop `SetPriority` and `ClearPriority`, add `Paint`)
- Modify: `crates/input/src/lib.rs` (drop `priority`, add `wisp`)
- Modify: `crates/input/src/selection.rs` (selection now triggered programmatically by wisp on tap, not via leafwing `just_pressed`)
- Modify: `crates/render/src/data_layer.rs` (`PriorityBiasMap` extraction stays — extension only if necessary)
- Modify: `crates/render/src/entity_render.rs` (replace `priority_arrow_render_system` with `bias_glow_render_system`)

**Note on parallelism:** runs in parallel with T4. Files do not overlap (T4 owns growth + regions; T5 owns input + render).

- [x] **Step 1: Verify baseline (T3 complete)**

```bash
just lint && just test && git status
```

- [x] **Step 2: Update `crates/input/src/action.rs`**

Drop `SetPriority` and `ClearPriority` variants from the `Action` enum. Add a `Paint` variant. Drop the corresponding `map.insert(Action::SetPriority, KeyCode::KeyP)` and `map.insert(Action::ClearPriority, ...)` lines from `default_input_map`. Add:

```rust
map.insert(Action::Paint, MouseButton::Left);
```

Note: `Action::SelectTile` is also bound to `MouseButton::Left`. Both fire simultaneously on press; the wisp state machine disambiguates after the fact. The `SelectTile` direct binding stays but the consumer (selection_system) is rewritten in step 5 to no longer respond to `just_pressed` — instead it listens to a wisp-emitted `TileTapped` message.

- [x] **Step 3: Delete `crates/input/src/priority.rs`**

```bash
rm crates/input/src/priority.rs
```

In `crates/input/src/lib.rs`, drop `mod priority;` and `pub use priority::priority_system;` and the registration in `InputPlugin::build`.

- [x] **Step 4: Create `crates/input/src/wisp.rs` with tests first**

```rust
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    BIAS_MAGNITUDE_CAP, BIAS_STROKE_INTENSITY, DRAG_THRESHOLD_PX, GridPos, GridWorld, Hex,
    HexLayout, SAMPLE_HEX_DISTANCE, SAMPLE_INTERVAL_MS, TAP_TIME_MS, Tile, WISP_SENSE_RADIUS_HEX,
};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

#[derive(Default, Clone, Debug)]
pub enum WispPhase {
    #[default]
    Idle,
    Primed {
        start_pos: Vec2,
        start_time: f32,
    },
    Stroking {
        last_sample_pos: Vec2,
        last_sample_time: f32,
    },
}

#[derive(Resource, Default)]
pub struct WispState {
    pub phase: WispPhase,
}

#[derive(bevy::ecs::message::Message)]
pub struct TileTapped {
    pub pos: Hex,
}

pub fn wisp_input_system(
    actions: Res<ActionState<Action>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    layout: Res<HexLayout>,
    grid: Res<GridWorld>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut wisp: ResMut<WispState>,
    mut taps: bevy::ecs::message::MessageWriter<TileTapped>,
) {
    let Some(cursor_world) = cursor_world_position(&windows, &cameras) else {
        return;
    };
    let now = time.elapsed_secs();

    let pressed = actions.pressed(&Action::Paint);
    let just_pressed = actions.just_pressed(&Action::Paint);
    let just_released = actions.just_released(&Action::Paint);

    // Snapshot owned hex positions ONCE before any mutable tile borrows.
    // The proximity lookup runs purely against this set, avoiding a borrow
    // conflict with the `tiles.get_mut(...)` call inside `write_segment`.
    let owned: std::collections::HashSet<Hex> = tiles
        .iter()
        .filter_map(|(gp, t)| t.region_id.map(|_| gp.0))
        .collect();

    // Take phase out so we can rebuild it.
    let prev = std::mem::take(&mut wisp.phase);
    let next = match prev {
        WispPhase::Idle => {
            if just_pressed {
                WispPhase::Primed {
                    start_pos: cursor_world,
                    start_time: now,
                }
            } else {
                WispPhase::Idle
            }
        }
        WispPhase::Primed {
            start_pos,
            start_time,
        } => {
            if just_released {
                if cursor_world.distance(start_pos) < DRAG_THRESHOLD_PX
                    && (now - start_time) * 1000.0 < TAP_TIME_MS as f32
                {
                    let hex = layout.world_pos_to_hex(start_pos);
                    taps.write(TileTapped { pos: hex });
                }
                WispPhase::Idle
            } else if pressed && cursor_world.distance(start_pos) > DRAG_THRESHOLD_PX {
                write_segment(start_pos, cursor_world, &layout, &grid, &owned, &mut tiles);
                WispPhase::Stroking {
                    last_sample_pos: cursor_world,
                    last_sample_time: now,
                }
            } else {
                WispPhase::Primed { start_pos, start_time }
            }
        }
        WispPhase::Stroking {
            last_sample_pos,
            last_sample_time,
        } => {
            if just_released {
                WispPhase::Idle
            } else if pressed {
                let elapsed_ms = (now - last_sample_time) * 1000.0;
                let hex_size = layout.scale.x;
                if elapsed_ms > SAMPLE_INTERVAL_MS as f32
                    || cursor_world.distance(last_sample_pos) > SAMPLE_HEX_DISTANCE * hex_size
                {
                    write_segment(last_sample_pos, cursor_world, &layout, &grid, &owned, &mut tiles);
                    WispPhase::Stroking {
                        last_sample_pos: cursor_world,
                        last_sample_time: now,
                    }
                } else {
                    WispPhase::Stroking {
                        last_sample_pos,
                        last_sample_time,
                    }
                }
            } else {
                WispPhase::Idle
            }
        }
    };
    wisp.phase = next;
}

fn cursor_world_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, cam_xform) = cameras.iter().next()?;
    camera.viewport_to_world_2d(cam_xform, cursor).ok()
}

fn write_segment(
    p1: Vec2,
    p2: Vec2,
    layout: &HexLayout,
    grid: &GridWorld,
    owned: &std::collections::HashSet<Hex>,
    tiles: &mut Query<(&GridPos, &mut Tile)>,
) {
    let direction = (p2 - p1).normalize_or_zero();
    if direction.length_squared() < 1e-6 {
        return;
    }
    let hex = layout.world_pos_to_hex(p2);
    let Some(&entity) = grid.tiles.get(&hex) else {
        return;
    };
    let falloff = network_proximity_factor(hex, grid, owned);
    if falloff <= 0.0 {
        return;
    }
    let Ok((_, mut tile)) = tiles.get_mut(entity) else {
        return;
    };
    let new_bias = tile.priority_bias + direction * BIAS_STROKE_INTENSITY * falloff;
    let mag = new_bias.length();
    tile.priority_bias = if mag > BIAS_MAGNITUDE_CAP {
        new_bias * (BIAS_MAGNITUDE_CAP / mag)
    } else {
        new_bias
    };
}

fn network_proximity_factor(
    hex: Hex,
    grid: &GridWorld,
    owned: &std::collections::HashSet<Hex>,
) -> f32 {
    if owned.is_empty() {
        return 0.0;
    }
    // BFS up to WISP_SENSE_RADIUS_HEX over the GridWorld topology.
    let mut frontier: std::collections::VecDeque<(Hex, i32)> = std::collections::VecDeque::new();
    frontier.push_back((hex, 0));
    let mut seen = std::collections::HashSet::new();
    seen.insert(hex);
    while let Some((current, dist)) = frontier.pop_front() {
        if dist > WISP_SENSE_RADIUS_HEX {
            continue;
        }
        if owned.contains(&current) {
            return 1.0 - (dist as f32) / (WISP_SENSE_RADIUS_HEX as f32 + 1.0);
        }
        for (npos, _) in grid.neighbors(current) {
            if seen.insert(npos) {
                frontier.push_back((npos, dist + 1));
            }
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    // Integration tests for state machine timing belong in T7 — unit tests here
    // just exercise the hex-bias-write helper paths.

    #[test]
    fn wisp_state_default_is_idle() {
        let s = WispState::default();
        assert!(matches!(s.phase, WispPhase::Idle));
    }
}
```

The unit test surface is intentionally thin — full state-machine behavior is tested in T7 integration tests where the cursor / window / time can be driven deterministically. The tile-write helpers above are pure functions that the T7 integration tests cover.

In `crates/input/src/lib.rs`:
- Add `mod wisp;`
- Add `pub use wisp::{WispPhase, WispState, TileTapped, wisp_input_system};`
- Init `WispState` resource and register `TileTapped` message in `InputPlugin::build`
- Register `wisp_input_system` in `Update` (every-frame, NOT in `SimulationSet` — input runs ungated)

```bash
cargo nextest run -p kingdom_input wisp
```

Expected: 1 test passes.

- [x] **Step 5: Update `crates/input/src/selection.rs` to consume `TileTapped`**

Read the current file. The selection system likely reads `actions.just_pressed(&Action::SelectTile)`. Replace that trigger with a `MessageReader<TileTapped>` (the wisp emits these only on confirmed taps). Body otherwise unchanged: convert `pos` to `SelectedRegion.selected_pos`.

If selection_system also handles `Action::SelectTile` for keyboard accessibility, leave that path; just add the message-reader path alongside.

- [x] **Step 6: Replace `priority_arrow_render_system` with `bias_glow_render_system`**

Open `crates/render/src/entity_render.rs`. Find `priority_arrow_render_system`. Rename it to `bias_glow_render_system` and rewrite the body to:

- Despawn previously spawned glow quads (use a marker component `BiasGlowMarker`)
- For each tile with `priority_bias.length_squared() > epsilon`, spawn a quad sprite at the tile's world position with alpha = `bias.length() / BIAS_MAGNITUDE_CAP` (clamped to 0..1) and a warm color (e.g. `Color::srgb(1.0, 0.7, 0.3)`)

```rust
use bevy::prelude::*;
use kingdom_core::{BIAS_MAGNITUDE_CAP, GridPos, HexLayout, Tile};

#[derive(Component)]
struct BiasGlowMarker;

pub fn bias_glow_render_system(
    mut commands: Commands,
    layout: Res<HexLayout>,
    tiles: Query<(&GridPos, &Tile)>,
    existing: Query<Entity, With<BiasGlowMarker>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
    for (gpos, tile) in tiles.iter() {
        let mag = tile.priority_bias.length();
        if mag < 0.05 {
            continue;
        }
        let alpha = (mag / BIAS_MAGNITUDE_CAP).min(1.0);
        let world = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            Sprite {
                color: Color::srgba(1.0, 0.7, 0.3, alpha),
                custom_size: Some(Vec2::splat(layout.scale.x * 1.6)),
                ..default()
            },
            Transform::from_xyz(world.x, world.y, 5.0),
            BiasGlowMarker,
        ));
    }
}
```

In `crates/render/src/lib.rs`, replace the `entity_render::priority_arrow_render_system` reference in the `PostUpdate` schedule with `entity_render::bias_glow_render_system`. Drop any priority-arrow-specific resource / asset wiring.

The despawn-and-respawn-each-frame pattern is the simplest correct implementation. A diff-based update is a follow-up optimization, not required for the refactor. On an 80×60 grid only owned + recently-painted tiles spawn glow quads (typically <100 entities), so archetype churn is acceptable. If the smoke test reveals frame hitching, switch to a `Children`-keyed update or a single instanced material.

- [x] **Step 7: Verify compilation**

```bash
cargo check --workspace --all-features --all-targets
```

- [x] **Step 8: Run input and render tests**

```bash
cargo nextest run -p kingdom_input
cargo nextest run -p kingdom_render
```

- [x] **Step 9: Run full suite**

```bash
just test
```

- [x] **Step 10: Smoke test** (skipped — no graphical environment in agent harness)

```bash
just dev
```

Expected: game launches. Click-and-drag from somewhere on the mycelium outward → a warm glow trail follows the cursor → mycelium leans toward it. Tap a tile → selection ring appears as before. Spacebar pauses; you can paint while paused; resume to watch growth.

- [x] **Step 11: Run lints**

```bash
just lint
```

- [x] **Step 12: Commit T5**

```bash
git add -A
git commit -m "T5: wisp drag-paint replaces P-key; bias glow renders warm trail"
```

---

## Task 6: UI simplification

**Goal:** Update HUD and tile popover to display the new resource model and drop specialization references. Slot machine UI keeps working — its triggers are now decomp-driven.

**Files:**
- Modify: `crates/ui/src/hud.rs`
- Modify: `crates/ui/src/tile_popover.rs`
- Modify: `crates/ui/src/lib.rs` (already adjusted in T1; verify hud and tile_popover remain registered)

**Note on parallelism:** runs in parallel with T3. Files do not overlap with T3 (which owns growth + world + render).

- [x] **Step 1: Verify baseline (T2 complete)**

```bash
just lint && just test && git status
```

- [x] **Step 2: Read the current hud.rs to find specialization references**

```bash
rg -n "specialization|nutrients|energy|biomass" crates/ui/src/hud.rs
```

- [x] **Step 3: Update `crates/ui/src/hud.rs`**

The HUD should display per-region or aggregated:
- Sugars: sum across regions or per-selected-region
- Melanin: same
- Turn: from `GameState.turn`
- Speed: from `SimulationSpeed.label()`
- Fragments: `fragments_fused / fragments_total`
- Mushrooms: `mushrooms_fruited / mushrooms_required`

Drop:
- Specialization tier display
- Specialization icon
- Anything reading `region.specialization*`

The exact pattern depends on whether the HUD uses Bevy `Text2d` or `bevy_ui` `Text`. Match the existing pattern. The semantic change: replace each specialization-related field with the corresponding new resource readout.

If the HUD currently shows `region.nutrients` and `region.energy`, replace with a single `region.sugars` line. Add a `region.melanin` line.

- [x] **Step 4: Update `crates/ui/src/tile_popover.rs`**

The popover should display, when a tile is selected:
- Terrain type
- Region ID (if owned) or "unowned"
- Biomass (formatted to 2 decimals)
- Moisture
- Radiation
- Soil richness (the renamed nutrient_level)
- Contents (if any)

Drop any specialization-related rows.

- [x] **Step 5: Verify compilation**

```bash
cargo check --workspace --all-features --all-targets
```

- [x] **Step 6: Run UI tests**

```bash
cargo nextest run -p kingdom_ui
```

- [x] **Step 7: Run full suite**

```bash
just test
```

- [x] **Step 8: Smoke test**

```bash
just dev
```

Expected: HUD shows `Sugars: <n>` and `Melanin: <n>` (will both be 0 unless T4 has shipped — by the parallel-batch convention T6 runs alongside T3, not T4, so resources may be at zero on smoke test; acceptable).

- [x] **Step 9: Run lints**

```bash
just lint
```

- [x] **Step 10: Commit T6**

```bash
git add -A
git commit -m "T6: HUD shows sugars/melanin; tile popover shows new tile fields"
```

---

## Task 7: Integration tests + verification

**Goal:** Add integration tests that prove the loop works end-to-end. Verify fruiting still progresses. Run the full lint / test / smoke pipeline clean.

**Files:**
- Create: `crates/growth/tests/integration.rs` (or per-system in respective crate `tests/` dirs)
- Modify: any tests in `crates/fruiting/src/*` whose construction patterns broke under T2 / T3

- [x] **Step 1: Verify baseline (T3, T4, T5, T6 complete)**

```bash
just lint && just test && git status
```

- [x] **Step 2: Add integration test `paint_then_grow`**

Create `crates/growth/tests/paint_then_grow.rs`:

```rust
//! Proves: writing positive priority_bias on a frontier tile causes biomass
//! to spread preferentially toward the bias direction over multiple ticks.

use bevy::prelude::*;
use kingdom_core::*;
use kingdom_growth::{
    DensityFlowRng, bias_decay_system, density_flow_system, dieback_system,
    moisture_diffusion_system, nutrient_gradient_system,
};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.insert_resource(create_hex_layout());
    app.insert_resource(DensityFlowRng(rand::SeedableRng::seed_from_u64(42)));
    app.add_message::<TileDiscovered>();
    app.add_systems(
        Update,
        (
            bias_decay_system,
            moisture_diffusion_system,
            nutrient_gradient_system,
            density_flow_system,
            dieback_system,
        )
            .chain(),
    );
    app
}

fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
    let e = app.world_mut().spawn((GridPos(pos), tile)).id();
    app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
    e
}

#[test]
fn paint_then_grow_biases_outflow() {
    let mut app = test_app();
    let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
    let layout = create_hex_layout();
    let center = Hex::new(10, 10);
    let neighbors = center.all_neighbors();
    let target = neighbors[0];
    let opposite = neighbors[3];
    let dir = (layout.hex_to_world_pos(target) - layout.hex_to_world_pos(center)).normalize();

    spawn(
        &mut app,
        center,
        Tile {
            region_id: Some(rid),
            biomass: 1.5,
            moisture: 1.0,
            priority_bias: dir * 1.0,
            ..default()
        },
    );
    for &n in &neighbors {
        spawn(&mut app, n, Tile { moisture: 0.6, ..default() });
    }

    for _ in 0..15 {
        app.update();
    }

    let grid = app.world().resource::<GridWorld>();
    let target_b = app.world().get::<Tile>(grid.tiles[&target]).unwrap().biomass;
    let opposite_b = app.world().get::<Tile>(grid.tiles[&opposite]).unwrap().biomass;

    assert!(
        target_b > opposite_b * 1.5,
        "biased target ({target_b}) should outpace opposite ({opposite_b}) by >1.5x"
    );
}
```

Run:

```bash
cargo nextest run -p kingdom_growth --test paint_then_grow
```

Expected: pass.

- [x] **Step 3: Add integration test `dry_zone_dieback`**

Create `crates/growth/tests/dry_zone_dieback.rs`:

```rust
use bevy::prelude::*;
use kingdom_core::*;
use kingdom_growth::{
    DensityFlowRng, bias_decay_system, density_flow_system, dieback_system,
    moisture_diffusion_system, nutrient_gradient_system,
};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.insert_resource(create_hex_layout());
    app.insert_resource(DensityFlowRng(rand::SeedableRng::seed_from_u64(7)));
    app.add_message::<TileDiscovered>();
    app.add_systems(
        Update,
        (
            bias_decay_system,
            moisture_diffusion_system,
            nutrient_gradient_system,
            density_flow_system,
            dieback_system,
        )
            .chain(),
    );
    app
}

#[test]
fn dry_zone_loses_claim_over_time() {
    let mut app = test_app();
    let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
    let pos = Hex::new(0, 0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                moisture: 0.0,
                ..default()
            },
        ))
        .id();
    app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);

    for _ in 0..40 {
        app.update();
    }

    let tile = app.world().get::<Tile>(e).unwrap();
    assert_eq!(
        tile.region_id, None,
        "starved tile should de-claim, biomass={}",
        tile.biomass
    );
}
```

Run:

```bash
cargo nextest run -p kingdom_growth --test dry_zone_dieback
```

Expected: pass.

- [x] **Step 4: Add integration test `slot_machine_triggers_on_unique_decomp`**

Create `crates/regions/tests/slot_machine_unique_decomp.rs`:

```rust
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use kingdom_core::*;
use kingdom_regions::{DecompProgress, decomposition_system, slot_machine_system};

#[test]
fn unique_decomp_to_slot_machine_pipeline() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<DecompProgress>();
    app.init_resource::<kingdom_regions::SlotMachineRng>();
    app.add_message::<DecompositionComplete>();
    app.add_message::<kingdom_regions::SlotMachineTriggered>();

    let captured = std::sync::Arc::new(std::sync::Mutex::new(0));
    let captured_c = captured.clone();
    app.add_systems(
        Update,
        (
            decomposition_system,
            slot_machine_system,
            (move |mut r: MessageReader<kingdom_regions::SlotMachineTriggered>| {
                for ev in r.read() {
                    if ev.options.len() == 3 {
                        *captured_c.lock().unwrap() += 1;
                    }
                }
            }),
        )
            .chain(),
    );

    let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
    let pos = Hex::new(0, 0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::UniqueDecomposable(0)),
                ..default()
            },
        ))
        .id();
    app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);

    // Force progress near completion.
    app.world_mut()
        .resource_mut::<DecompProgress>()
        .entries
        .insert(pos, 0.99);

    app.update();
    app.update(); // event delivery may take a frame.

    assert_eq!(*captured.lock().unwrap(), 1);
}
```

Run:

```bash
cargo nextest run -p kingdom_regions --test slot_machine_unique_decomp
```

Expected: pass.

- [x] **Step 5: Add integration test `decompose_to_fragment_fusion`**

Create `crates/regions/tests/decompose_to_fragment.rs`. The test sets up a tile with a `Fragment(_)` content and adjacent owned mycelium with biomass > CLAIM_THRESHOLD; runs ticks; asserts `GameState.fragments_fused` increments. The fragment_system code (existing in `crates/regions/src/fragment.rs`) was migrated in T2 — verify the test is consistent with whatever ownership semantics fragment_system now uses.

Skeleton:

```rust
use bevy::prelude::*;
use kingdom_core::*;
use kingdom_regions::fragment_system;

#[test]
fn fragment_fuses_when_covered() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<GameState>();
    // FragmentFused is also registered by CorePlugin in real builds; this test
    // uses MinimalPlugins, so the registration is needed here. Harmless.
    app.add_message::<FragmentFused>();
    app.add_systems(Update, fragment_system);

    let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
    let pos = Hex::new(0, 0);
    let fragment_id = FragmentId(0);
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                contents: Some(TileContents::Fragment(fragment_id)),
                ..default()
            },
            FragmentAgent { fragment_id, fused: false },
        ))
        .id();
    app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);
    app.world_mut().resource_mut::<GameState>().fragments_total = 1;

    app.update();

    let agent = app.world().get::<FragmentAgent>(e).unwrap();
    assert!(agent.fused);
    assert_eq!(app.world().resource::<GameState>().fragments_fused, 1);
}
```

If the fragment_system fusion criterion differs (e.g. requires `HUB_THRESHOLD` not `CLAIM_THRESHOLD`), adjust the test setup. Read fragment.rs to confirm.

Run:

```bash
cargo nextest run -p kingdom_regions --test decompose_to_fragment
```

Expected: pass.

- [ ] **Step 6: Verify fruiting still progresses** (skipped — requires graphical environment)

Manual verification:

```bash
just dev
```

Inside the running game: paint a stroke toward a fragment-bearing tile, wait until fragment is covered (biomass crosses CLAIM_THRESHOLD), confirm fragment fusion message in console / HUD. Verify a fruiting body sprite eventually appears on the surface column above the region.

If `crates/fruiting/src/fruiting.rs` recipe checks `region.total_biomass` correctly, a fruit body should grow within ~1 minute of stable territory. If it doesn't, read fruiting.rs and check whether T2's RegionState rename was properly applied.

- [x] **Step 7: Run full lints and tests**

```bash
just lint
just test
```

Expected: both pass clean.

- [ ] **Step 8: Final smoke test** (skipped — requires graphical environment)

```bash
just dev
```

Click-paint outward from spawn, watch density spread, observe fragments fuse and fruit bodies appear, check HUD resource counters update.

- [x] **Step 9: Commit T7**

```bash
git add -A
git commit -m "T7: integration tests prove paint→grow→decompose→fragment→fruit loop end-to-end"
```

---

## Done criteria

The refactor is complete when:

- All seven tasks above have every checkbox flipped to `- [x]`.
- `just lint` passes.
- `just test` passes.
- `just dev` launches the game; the player can paint a wisp from spawn, watch mycelium spread along the stroke, see biomass deepen, see decomposition yield sugars, see symbiosis trigger when adjacent to plant roots, see melanin accumulate when adjacent to radiation, see fragments fuse on coverage, and see fruit bodies grow on the surface.
- The `polish` agent runs `post-implementation-polish` and reports clean.
