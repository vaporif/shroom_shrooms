# Density-flow growth refactor

Status: ready for review
Date: 2026-05-07
Scope: simulation core, input, rendering, UI

## Goal

Replace the existing tip-agent growth model with a continuous density-flow model driven by player-painted directional bias. Strip the specialization, abilities, slot-machine-mutation, combat, and rival-AI subsystems. Introduce the design-doc resource model (water per tile, sugars and melanin per region) with three sources of sugars: decomposition of dead organic matter, symbiosis with plant roots, and (future) cards. Replace the P-key priority stamp with a mouse-drag "wisp" stroke that decays over time.

The end state: a thinner, more thematically coherent simulation that expresses the "single distributed mind" framing as actual code, without internal hidden agents. Specialization can be re-introduced later as a per-region modifier on the flow rule if the gameplay layer needs it. The game stays playable end-to-end after the refactor; the win condition (fragments fused plus mushrooms fruited) is unchanged.

## Background and constraints

The current simulation models growth as discrete `HyphalTip` entities that hop one hex per tick along a combination of nutrient gradient and player-set priority bias. The bias is set by selecting a tile and pressing P; it persists until cleared. Eight specializations modify tip behavior, ability bar, and per-region passives. A rival fungus AI competes for territory.

The design conversations leading to this spec converged on three observations. The tip-agent abstraction is invisible to the player and exists primarily as the integration point for specialization tip-modifiers; if specialization is dropped, the abstraction has no defenders. The P-key stamp is functional but flat — it cannot express direction or intensity, and it persists indefinitely. The current four-resource model in code (`nutrients`, `energy`, `biomass`, `specialization_investment`) does not match the design document's three-resource model (water, sugars, melanin), and the gap blocks several design-doc-aligned features (paths, decomposition mechanics, mycorrhizal symbiosis).

The user has opted to drop the specialization layer entirely, disable the rival AI, and switch to the design-doc resource model. Cards are not in scope. Slot machine stays, retriggered by `UniqueDecomposable` consumption rather than specialization tier completion; reward effects remain deferred. Mutation effects are parked.

## Architecture

The simulation core changes from "agents hopping on a tile grid" to "density field flowing across a tile grid, driven by player-painted bias and tile-local resources." Specialization, abilities, slot machine reward effects, mutations, combat, and rival AI are removed. Organisms (plants, fauna, bacteria, neutral fungi) and fragments / fruiting stay.

```
[Input]                       [Simulation: SimulationSet, tick-gated]
   wisp ── paint ──> bias  ─┬─> density_flow ─> Tile.biomass +/-
                            ├─> dieback      ─> shrinks low-moisture density
                            ├─> moisture     ─> diffusion + sources
                            ├─> decomposition─> Region.sugars +
                            ├─> symbiosis    ─> Region.sugars +
                            ├─> melanin      ─> Region.melanin +
                            └─> bias_decay   ─> Tile.priority_bias *= 0.95
                  organisms ─> environmental events (existing, untouched)
                  fragments ─> discovery + fruiting (existing, simplified)
```

| Crate | Change |
|---|---|
| `kingdom_core` | Strip `SpecializationType`, `abilities.rs`, spec fields on `RegionState`. Drop `Occupant` enum. Add `Tile.radiation`. Rename `Tile.nutrient_level` to `Tile.soil_richness`. Add new constants. |
| `kingdom_growth` | Replace `tip.rs` + `nutrient.rs` + `decay.rs` with `density_flow.rs`, `dieback.rs`, `moisture.rs`, `bias_decay.rs`, `decomposition.rs`, `symbiosis.rs`, `melanin.rs`. Keep `nutrient_gradient_system` (it now reads `soil_richness`). |
| `kingdom_input` | Replace `priority.rs` with `wisp.rs`. Drop `specialization_input.rs`. Add `Action::Paint`. |
| `kingdom_regions` | Drop `specialization.rs`. Keep `slot_machine.rs` (re-triggered from decomposition). Keep `fragment.rs`. Refactor `discovery.rs`: delete `explorer_discovery_system` and `researcher_study_system`; universalise `decomposer_discovery_system` into a `decomposition_system`. `mutation.rs` is preserved (dormant — slot-machine reward pipeline runs end-to-end but applies no effect, per the deferred reward design). |
| `kingdom_ai` | Drop `RivalAiPlugin`, `combat.rs`. Keep `OrganismsPlugin`, `EnvironmentPlugin`. |
| `kingdom_ui` | Drop `ability_bar.rs`, `spec_picker.rs`. Keep `slot_machine_ui.rs`. Simplify `hud.rs`, `tile_popover.rs`. |
| `kingdom_render` | Drop tip rendering. Drop rival rendering (`RivalBranchGraph` resource and `extract_rival_branch_graph` extraction in `data_layer.rs`; rival-graph parameter and `group_rival_nodes_by_id` helper in `network_render.rs`). Update `extract_branch_graph` for biomass-driven edges. Replace priority arrow renderer with bias glow. |
| `kingdom_fruiting` | Verify recipes read `RegionState.total_biomass`. No structural change expected. |
| `kingdom_world` | Update `region_tracking_system` to use the new ownership semantics. Terrain generation gains a radiation-seed pass: each `TerrainType::Ruin` tile sets its own `radiation` to a value in `0.6..=1.0` (uniform); every other tile within 2-hex distance of any Ruin gets `radiation = 0.4 * falloff` where `falloff = 1.0 - distance/2`; all other tiles get `radiation = 0.0`. Use the existing world generation `LaunchConfig.seed` so radiation is deterministic per seed. |

The plugin order in `bin/src/plugins.rs` removes `AiPlugin` (or splits it: `OrganismsPlugin` + `EnvironmentPlugin` registered directly). Everything else registers as before.

## Data model

### `Tile`

```rust
pub struct Tile {
    pub terrain: TerrainType,
    pub region_id: Option<RegionId>,    // REPLACES `occupant: Occupant`
    pub biomass: f32,                   // 0.0..=BIOMASS_CAP; ownership at >= CLAIM_THRESHOLD
    pub moisture: f32,                  // 0.0..=1.0
    pub radiation: f32,                 // NEW; 0.0..=1.0; written by terrain gen
    pub soil_richness: f32,             // RENAMED from `nutrient_level`
    pub nutrient_gradient: Vec2,        // recomputed each tick
    pub priority_bias: Vec2,            // decayed each tick by bias_decay_system
    pub discovered: bool,
    pub contents: Option<TileContents>,
}
```

The `Occupant` enum is dropped. Without rivals, "occupied" collapses to `region_id.is_some() && biomass >= CLAIM_THRESHOLD`. Every `is_player()` and `match Occupant::Player(rid)` call site migrates to `region_id` checks. `nutrient_level` is renamed to `soil_richness` because the new region-level `sugars` is a different concept (energy currency, not soil capacity).

`TileContents` is unchanged. All seven variants (`OrganicMatter`, `Mineral`, `Artifact`, `Fragment`, `UniqueDecomposable`, `NeutralFungus`, `PlantRoot`) stay; the decomposition system reads them.

### `RegionState`

```rust
pub struct RegionState {
    pub region_id: RegionId,
    pub sugars: f32,            // REPLACES nutrients + energy
    pub melanin: f32,           // NEW
    pub tile_count: u32,
    pub total_biomass: f32,     // RENAMED + redefined: sum of Tile.biomass across owned tiles
}
```

Stripped: `specialization`, `target_specialization`, `specialization_investment`, `nutrient_bonus`. The two old resource pools (`nutrients`, `energy`) collapse into `sugars`. `biomass` at the region level was a separate accumulator; it is now a summary of per-tile biomass, recomputed each tick — single source of truth lives on tiles.

### Components

| Component | Status |
|---|---|
| `HyphalTip` | Removed. Density flow replaces it. |
| `FaunaAgent`, `BacteriaColonyAgent`, `PlantRootAgent`, `NeutralFungusAgent`, `FragmentAgent`, `FruitingBody`, `MushroomEntity`, `OrganismSpriteLink`, `SelectedRegion`, `GridPos` | Kept, no field changes. |

### New constants

| Constant | Default | Used by |
|---|---|---|
| `CLAIM_THRESHOLD` | 0.3 | density_flow, region_tracking, fruiting |
| `HUB_THRESHOLD` | 1.0 | fruiting recipes |
| `BIOMASS_CAP` | 2.0 | density_flow |
| `BIAS_DECAY` | 0.95 | bias_decay |
| `BIAS_STROKE_INTENSITY` | 0.5 | wisp input |
| `BIAS_MAGNITUDE_CAP` | 1.5 | wisp input |
| `WATER_GROWTH_COST` | 0.05 | density_flow |
| `DECOMP_RATE` | 0.02 | decomposition |
| `SUGAR_FROM_DECOMP` | 0.5 | decomposition |
| `SUGAR_FROM_SYMBIOSIS` | 0.1 | symbiosis |
| `MELANIN_FROM_RADIATION` | 0.1 | melanin |
| `AUTONOMOUS_FLOW_WEIGHT` | 0.1 | density_flow (baseline outflow rate, no bias or gradient) |
| `BIASED_FLOW_WEIGHT` | 0.6 | density_flow (multiplier on player bias alignment) |
| `GRADIENT_FLOW_WEIGHT` | 0.1 | density_flow (multiplier on nutrient gradient alignment) |
| `FLOW_NOISE` | 0.15 | density_flow |
| `MIN_FLOW_DENSITY` | 0.05 | density_flow |
| `DIEBACK_THRESHOLD` | 0.05 | dieback |
| `DIEBACK_RATE` | 0.95 | dieback |
| `DRAG_THRESHOLD_PX` | 6.0 | wisp input |
| `TAP_TIME_MS` | 150 | wisp input |
| `SAMPLE_INTERVAL_MS` | 50 | wisp input |
| `SAMPLE_HEX_DISTANCE` | 0.5 | wisp input |
| `WISP_SENSE_RADIUS_HEX` | 5 | wisp input |

These live as `pub const` in `crates/core/src/constants.rs`. Promoting to a runtime `Config` resource for hot-reload tuning is a follow-up if needed.

### Messages

`TurnAdvanced`, `TileDiscovered`, `DecompositionComplete`, `FragmentFused`, `SlotMachineTriggered`, `NeutralFungiMerged` all stay. `StudyComplete` is **removed** (its only producer, `researcher_study_system`, is deleted along with specialization). `SlotMachineTriggered` re-publishes from decomposition completion instead of specialization tier-up — same shape, different publisher. `TileDiscovered` re-publishes from `density_flow_system` claim events instead of specialization-gated discovery systems.

## Systems

Eleven systems chain inside `SimulationSet`. The `wisp_input_system` runs every frame ungated; the rest are tick-gated.

| # | System | Crate | Status | Purpose |
|---|---|---|---|---|
| 1 | `wisp_input_system` | input | NEW | Reads mouse drag, writes `priority_bias` to tiles under stroke. Every-frame, ungated. |
| 2 | `bias_decay_system` | growth | NEW | `tile.priority_bias *= BIAS_DECAY`; zeroes when below epsilon. |
| 3 | `moisture_diffusion_system` | growth | NEW | Water terrain stays at 1.0 (source); passable tiles diffuse toward neighbors. |
| 4 | `nutrient_gradient_system` | growth | KEPT | Recomputes `nutrient_gradient` from `soil_richness` diffs. |
| 5 | `density_flow_system` | growth | NEW (replaces `tip` + `decay`) | Two-phase: compute outflows, apply deltas. Spends water, claims neighbors. |
| 6 | `dieback_system` | growth | NEW | Tiles with low moisture or surplus density shrink; below threshold → de-claim. |
| 7 | `decomposition_system` | regions | NEW (subsumes spec-gated `decomposer_discovery_system`) | Per-tile progress under owned biomass; fires `DecompositionComplete` and `SlotMachineTriggered`. |
| 8 | `symbiosis_system` | growth | NEW | Owned tile adjacent to `PlantRoot` → sugars to region, moisture from region. |
| 9 | `melanin_system` | growth | NEW | Owned tile with radiation → melanin to region; tile radiation slowly depletes. |
| 10 | `region_tracking_system` | world | MODIFIED | Connected components by `biomass ≥ CLAIM_THRESHOLD`, assigns `region_id`. |
| 11 | `slot_machine_system` | regions | KEPT (re-triggered) | Reads `SlotMachineTriggered` now sourced from decomposition. |

Existing untouched: `fragment_system`, `fruiting` recipe checks, organism systems, environment events.

### Density flow rule (system 5)

Two-phase to avoid order dependence.

**Phase 1: compute outflows.** For each tile `T` where `biomass > MIN_FLOW_DENSITY`, for each passable neighbor `N` whose `region_id` is `None` or equals `T.region_id`:

```
direction = unit_vec(N.world - T.world)
bias_score     = max(0, T.priority_bias.dot(direction))
gradient_score = max(0, T.nutrient_gradient.dot(direction))

weight = AUTONOMOUS_FLOW_WEIGHT
       + BIASED_FLOW_WEIGHT   * bias_score
       + GRADIENT_FLOW_WEIGHT * gradient_score
weight *= 1.0 + (rng() - 0.5) * FLOW_NOISE
```

Total outflow from `T` is capped by `0.1 * T.biomass` (don't drain more than ten percent per tick) and by `T.moisture / WATER_GROWTH_COST` (water budget). Distribute the budget across neighbors by `weight / total_weight`. Record delta per neighbor.

**Phase 2: apply.** Sum incoming deltas per tile; cap at `BIOMASS_CAP`. If `N.region_id` was `None` and the sum pushes biomass past `CLAIM_THRESHOLD`, assign `region_id` to the largest-contributing source region. Subtract water spent on growth from each source's `moisture`.

Anastomosis is implicit: two flows from the same region into one empty hex sum naturally. Cross-region flow is blocked by the neighbor filter in MVP.

### Dieback (system 6)

Runs after density flow. For each tile `T` with `biomass > 0`: if `moisture < DIEBACK_THRESHOLD`, multiply biomass by `DIEBACK_RATE`. If biomass falls below `CLAIM_THRESHOLD`, clear `region_id`. If biomass falls below epsilon, snap to zero.

### Decomposition (system 7)

Per-tile progress accumulates while `tile.biomass >= CLAIM_THRESHOLD` and `tile.contents` is `OrganicMatter` or `UniqueDecomposable`. Each tick adds `DECOMP_RATE` to progress and credits `SUGAR_FROM_DECOMP * DECOMP_RATE` sugars to the region. On reaching 1.0, contents clear, `soil_richness` increases by 0.2, `DecompositionComplete` fires, and `UniqueDecomposable` additionally fires `SlotMachineTriggered`.

Progress storage: a `HashMap<Hex, f32>` resource in `kingdom_regions` (matches the existing `DecompProgress` resource).

### Symbiosis (system 8)

For each owned tile `T` adjacent to a tile `N` where `N.contents == PlantRoot(_)`: if `T.biomass >= CLAIM_THRESHOLD` and `T.moisture > 0.3`, set the plant agent's `trade_active` to true, add `SUGAR_FROM_SYMBIOSIS` to the region, and subtract `SUGAR_FROM_SYMBIOSIS * 0.3` from `T.moisture` as the water cost of the trade. Reuses `PlantRootAgent.trade_active`, which already exists.

### Melanin (system 9)

For each owned tile `T` where `T.biomass >= CLAIM_THRESHOLD` and `T.radiation > 0`: add `MELANIN_FROM_RADIATION * T.radiation` to `region.melanin`, and subtract a small fraction (10 percent of that yield) from `T.radiation`. The slow tile cleansing is what makes long-term presence in radiated zones gradually heal them.

### Bias decay (system 2)

For each tile, multiply `priority_bias` by `BIAS_DECAY`. If the magnitude falls below epsilon, snap to `Vec2::ZERO`.

### What is not a system

Bias painting itself (writing into `Tile.priority_bias`) is input, not simulation. It runs every frame, ungated by the tick. Simulation systems just read the bias each tick.

## Input — wisp drag mechanics

Wisp painting drives a state machine that writes `priority_bias` into tiles under the cursor as a stroke unfolds. The simulation reads that bias on the next tick.

### Action mapping

| Action | Binding | Status |
|---|---|---|
| `Action::Paint` | Left mouse button (held) | NEW |
| `Action::SetPriority` | (was P-key) | REMOVED |
| `Action::ClearPriority` | (was Shift+P) | REMOVED |
| `Action::SelectTile` | Left mouse button (tap-disambiguated) | unchanged binding |
| Pan, zoom, speed, pause | unchanged | unchanged |

Both `Paint` and `SelectTile` bind to left mouse button. Disambiguation happens in code via the wisp state machine, not in the binding map.

### State machine (per frame)

```
WispState = Idle | Primed { start_pos, start_time } | Stroking { last_sample_pos, last_sample_time }

on Paint just_pressed at cursor C:
    state = Primed { C, now }

on Paint held at cursor C while Primed:
    if distance(C, start_pos) > DRAG_THRESHOLD_PX:
        state = Stroking { C, now }
        write_bias_segment(start_pos, C)

on Paint held at cursor C while Stroking:
    if now - last_sample_time > SAMPLE_INTERVAL_MS
       || distance(C, last_sample_pos) > SAMPLE_HEX_DISTANCE * hex_size:
        write_bias_segment(last_sample_pos, C)
        state = Stroking { C, now }

on Paint released while Primed:
    if now - start_time < TAP_TIME_MS && distance(cursor, start_pos) < DRAG_THRESHOLD_PX:
        forward_to_selection_system(start_pos)
    state = Idle

on Paint released while Stroking:
    state = Idle
```

The `SelectTile` action becomes a derived signal emitted on a recognized tap, not a direct binding. This is the only correct way to share one button between tap and drag.

### Bias write per stroke segment

Given two cursor positions `P1` and `P2` in world space:

```
hex = layout.world_pos_to_hex(P2)
direction = (P2 - P1).normalize_or_zero()

if let Some(tile_entity) = grid.tiles.get(&hex):
    let mut tile = tiles.get_mut(tile_entity)?;
    let falloff = network_proximity_factor(hex, &grid);
    tile.priority_bias = clamp_magnitude(
        tile.priority_bias + direction * BIAS_STROKE_INTENSITY * falloff,
        BIAS_MAGNITUDE_CAP,
    );
```

`network_proximity_factor` returns 1.0 if the hex is within 1 step of an owned tile, falling linearly to 0.0 at `WISP_SENSE_RADIUS_HEX` hexes away. Strokes outside that range write nothing — the body desires only what it can sense.

`clamp_magnitude` lets multiple strokes stack but caps the maximum pull. Bias decay erodes it each tick; strokes refresh it.

### Pause behaviour

`wisp_input_system` runs every frame regardless of `SimulationSpeed`. Painting while paused writes bias as normal; the bias just sits on tiles until simulation resumes and the flow systems read it on the next tick.

### Resources added

```rust
// crates/input/src/wisp.rs
#[derive(Resource, Default)]
pub struct WispState {
    pub phase: WispPhase,
}

pub enum WispPhase {
    Idle,
    Primed { start_pos: Vec2, start_time: f32 },
    Stroking { last_sample_pos: Vec2, last_sample_time: f32 },
}
```

Single resource. No per-stroke entity needed for MVP.

### Trackpad notes

The state machine works identically for trackpad single-finger drag. Two-finger drag remains bound to camera pan in `crates/input/src/camera.rs` — different input channel, no conflict. Pinch-to-zoom maps to scroll-wheel events on macOS by default; the existing zoom binding handles it. Battery on idle: existing pause-aware tick gating already handles this; consider `bevy::winit::WinitSettings::desktop_app()` for power saving in a follow-up.

## Rendering

The two-layer separation stays — data extraction in `data_layer.rs`, presentation in shaders and materials. Three things move.

### Network rendering — biomass drives thickness

`network_render::network_render_system` already renders multi-strand Catmull-Rom splines from a `BranchGraph` of owned tile pairs and already scales strand thickness with biomass. The change is upstream: `data_layer::extract_branch_graph` builds edges between any two adjacent tiles where both have `biomass >= CLAIM_THRESHOLD` (currently uses `Occupant::Player` — straight migration to `region_id.is_some()` plus the threshold check). Continuous biomass produces continuous strand thickness — the discrete-hop concern dissolves at the render layer.

### Tip rendering — removed

```
data_layer::extract_tip_positions       → DELETE
data_layer::TipPositions resource       → DELETE
entity_render::tip_render_system        → DELETE
```

The visible "growing front" is whichever tile within the network has the lowest biomass — emergent, not tracked.

### Bias glow — new

Today, `entity_render::priority_arrow_render_system` renders arrows pointing along `priority_bias`. Change semantics to a soft glow heatmap: `PriorityBiasMap` already extracts per-tile bias magnitudes; replace the arrow renderer with a quad per biased tile whose alpha and warm hue scale to `bias.length() / BIAS_MAGNITUDE_CAP`. Glow fades naturally as decay shrinks magnitudes — no separate animation. Optional debug toggle (F-key) to render arrows on top during development.

### UI

| File | Action |
|---|---|
| `ability_bar.rs` | DELETE |
| `spec_picker.rs` | DELETE |
| `slot_machine_ui.rs` | KEEP — re-triggered by decomposition |
| `hud.rs` | SIMPLIFY — drop specialization tier display; add `sugars` and `melanin` readouts |
| `tile_popover.rs` | SIMPLIFY — drop specialization fields; show terrain, biomass, moisture, radiation, region_id, contents |
| `game_screens.rs` | KEEP unchanged |

The HUD resource panel becomes:

```
[Sugars: 24]  [Melanin: 3]  [Turn: 142]  [▶▶ 2x]  [Fragments: 2/5]  [Mushrooms: 0/3]
```

### Atmosphere, terrain, organisms, region highlight

All untouched.

## Gameplay loop

```
spawn at center
   ↓
paint wisp outward → density flows along bias
   ↓
mycelium covers organic matter
   → decomposition → sugars trickle
   → if UniqueDecomposable → SlotMachineTriggered → spin (reward TBD)
   ↓
mycelium reaches plant roots → symbiosis
   → sugars (steady)
   → green path identity
   ↓ OR
mycelium reaches radiation → melanin accumulates
   → black path identity
   ↓
sugars + melanin enable big moves: long jumps, hub formation, fruit-body recipes
   ↓
fragments covered (biomass >= CLAIM_THRESHOLD on fragment tile)
   → fragment_system marks fused, increments GameState.fragments_fused
   → fruit-body begins on the region
   ↓
fruit-body completes → MushroomEntity spawns
   → GameState.mushrooms_fruited increments
   ↓
all fragments fused AND mushrooms_fruited >= mushrooms_required
   → GamePhase::Victory
```

### Win condition — unchanged

`GameState::victory()` already checks `fragments_fused >= fragments_total` and `mushrooms_fruited >= mushrooms_required`. No structural change.

### Fail state — none in MVP

`GamePhase::Defeat` is unused by any current system. Leave it that way for now. Drought is a setback (network shrinks, player can re-spread), not a loss. Add a real fail state later if playtesting shows the loop needs the tension.

### Slot machine in the loop

When `decomposition_system` completes a `UniqueDecomposable`: clear contents, fire `DecompositionComplete`, fire `SlotMachineTriggered { region_id }`. `slot_machine_system` consumes, spins, publishes `SlotResult`. `mutation_system` (the current consumer) remains dormant for MVP — no effects applied, per the deferred reward design.

### Discovery and fog of war

`Tile.discovered` stays. `decomposer_discovery_system` becomes a universal `decomposition_system` (handles both contents-decomposition and reveal). `explorer_discovery_system` and `researcher_study_system` are **deleted**; the `StudyProgress` resource and `StudyComplete` message are also removed (their only producer was the researcher system). Fog-of-war reveal happens as a side effect of region growth: when a tile crosses `CLAIM_THRESHOLD` in `density_flow_system`, mark it and all hex-distance-1 neighbors `discovered = true`. Existing `TileDiscovered` message is republished from this hook so listeners (UI, organism systems) keep working.

## Testing strategy

The existing test pattern (`MinimalPlugins`, `test_app()` helpers, `spawn_tile_at()`, `cargo nextest run -p <crate>`) carries forward. No new infrastructure.

### Tests deleted

| Test | File | Why |
|---|---|---|
| `tip_moves_toward_nutrient_gradient`, `tip_dies_when_no_passable_neighbors`, `tip_anastomosis_boosts_biomass` | `crates/growth/src/tip.rs` | `HyphalTip` removed |
| `decomposer_region_produces_nutrients` | `crates/growth/src/nutrient.rs` | Specialization gone; replaced by universal decomposition test |
| `transporter_moves_nutrients_between_regions` | `crates/growth/src/nutrient.rs` | Specialization gone; no replacement |
| `rival_expands_into_empty_neighbors` | `crates/ai/src/rival.rs` | Rival AI gone; whole file deleted |
| `p_key_sets_bias_around_selected_tile`, `shift_p_clears_bias`, `p_with_no_selection_is_noop` | `crates/input/src/priority.rs` | P-key removed |

### Unit tests added

Each new system gets at least one happy-path test and one boundary test. Coverage list:

- `bias_decay_system`: non-zero bias multiplies; bias below epsilon snaps to zero.
- `moisture_diffusion_system`: water terrain stays at 1.0; adjacent tile gains moisture; far tile unchanged; moisture clamps non-negative.
- `density_flow_system`: flow follows bias direction; flow proportional to source biomass; flow consumes water; biased flow exceeds autonomous; biomass caps at `BIOMASS_CAP`; empty tile claimed at `CLAIM_THRESHOLD`; cross-region tiles not entered.
- `dieback_system`: low moisture shrinks biomass; below `CLAIM_THRESHOLD` clears `region_id`; below epsilon snaps to zero.
- `decomposition_system`: owned tile plus organic matter accumulates progress; progress to 1.0 clears contents, raises `soil_richness`, fires `DecompositionComplete`; `UniqueDecomposable` fires `SlotMachineTriggered`; non-owned tile no progress.
- `symbiosis_system`: adjacent plant root adds sugars; low moisture blocks trade; moisture decreases on trade; `PlantRootAgent.trade_active` flips.
- `melanin_system`: radiated owned tile adds melanin; tile radiation decreases; non-radiated tile contributes nothing.
- `region_tracking_system`: connected biomass-claimed tiles form one region; disconnected groups form separate; tile below `CLAIM_THRESHOLD` excluded.
- `wisp_input_system`: tap forwards to selection (no stroke); drag past threshold enters stroke; stroke writes bias along direction; bias clamped at cap; out-of-network falloff zero; stroke works while paused.

### Integration tests added

| Test | What it proves |
|---|---|
| `paint_then_grow` | Paint bias on tile, run 10 ticks → biomass spreads in painted direction with measurable preference. |
| `decompose_to_fragment_fusion` | Place fragment tile, paint over it, run ticks → `fragments_fused` increments. |
| `dry_zone_dieback` | Claim wide area, set moisture=0 in zone, run ticks → biomass shrinks, tiles de-claim. |
| `symbiosis_provides_sugars` | Plant root + adjacent painted growth → `region.sugars > 0` after N ticks. |
| `radiation_provides_melanin` | Radiation tile + adjacent painted growth → `region.melanin > 0` after N ticks. |
| `slot_machine_triggers_on_unique_decomp` | Place `UniqueDecomposable`, paint over, run ticks → `SlotMachineTriggered` fires once. |
| `path_split_no_rival_interference` | Two non-adjacent paint regions → grow independently, no cross-pollution. |

## Decisions and trade-offs

### Density flow over tip-agent extension

**Pros:** Matches the "single distributed mind" theme as actual code, no internal hidden agents. Anastomosis and dieback are symmetric (inverse of growth). Continuous density renders smoothly without discrete-hop artifacts. Removes the awkward intermediate tip-as-entity layer.

**Cons:** Big-bang refactor of the simulation core. Existing tip tests rewritten from scratch. Tuning the flow rule requires playtesting iteration (bias weight, gradient weight, noise, water cost).

**Why we chose this:** The user opted to drop the specialization layer. With specialization gone, the tip-as-entity abstraction had no defenders — its only justification was being the integration point for eight specialization tip-modifiers. Once those are removed, the tip layer is dead weight between input and tile state, and density flow becomes the strictly cleaner expression. See ADR for full decision context.

### Three resources (water, sugars, melanin) over four (adding minerals)

**Pros:** Three resources is enough dimensionality for meaningful trade-offs without overwhelming the player or the UI. Folding minerals into sugars (energy = work) is biologically defensible and matches design discussions.

**Cons:** Loses the "stone-breaking is a distinct resource cost" texture the design doc originally had. Dead organic matter no longer yields a unique resource.

**Why we chose this:** Stone-breaking can still cost sugars at a higher rate, preserving the texture without an extra UI dimension. The four-resource model added per-tile UI complexity (mineral readouts) for marginal gameplay differentiation. Three is the sweet spot for an MVP.

### Decomposition as universal mechanic, not specialization perk

**Pros:** Matches real fungal biology — saprotrophic activity is the default, not a class. Without specialization, every region needs decomposition to access the primary early-game energy source. Slot machine retrigger from `UniqueDecomposable` consumption gives a reason to spread toward dead matter beyond just sugars.

**Cons:** All regions decompose at the same rate; no specialization variety in decomposition speed.

**Why we chose this:** Specialization is being removed regardless. Universal decomposition is the natural default. Variety can return as cards or per-region modifiers later.

### Wisp drag input over P-key stamp

**Pros:** Tactile, expressive (direction, intensity, length), trackpad-friendly via single-finger drag, decay forces active intent (paint as verb, not state). Aligns with "single mind willing it" theme.

**Cons:** Tap-vs-drag disambiguation requires care. Right-click dissuasion is dropped to stay trackpad-friendly. Mouse-only — no keyboard-only fallback for accessibility (deferred).

**Why we chose this:** P-key stamp is functional but flat. Wisp painting is the natural maturation of the same `priority_bias` data field, with stronger gameplay-feel payoff. Trackpad mappings preserve full functionality without right-click.

### Drop `Occupant` enum entirely

**Pros:** Without rivals, `Occupant` collapses to a one-of-two shape (`Empty` or `Player(RegionId)`) that isn't worth the type ceremony. `Option<RegionId>` is simpler and more idiomatic. All call sites migrate mechanically.

**Cons:** Wide migration surface — every `is_player()`, every `match Occupant::*` call site needs updating.

**Why we chose this:** The migration is mechanical, not architectural. The simpler model pays back the migration cost in every future read.

### Slot machine retained, mutation effects deferred

**Pros:** Slot machine UI/animation already exists and is good. Repurposing the trigger from spec-tier to decomp-completion costs almost nothing. Keeps the surprise-reward flavor of the original game.

**Cons:** Reward effects (mutation system) are dormant — slot machine spins but grants nothing in MVP. This is visible to players (they see the spin and notice no effect).

**Why we chose this:** The user explicitly deferred reward design. Stripping the slot machine entirely would lose work and require rebuilding when rewards are designed; leaving it dormant is the cheapest option.

## Execution Strategy

**Subagents.** Every task runs in a fresh dispatched agent. The executor batches independent tasks into parallel dispatches and chains dependent tasks sequentially based on the dependency graph below. The refactor touches a shared simulation core, so most tasks are sequential; only T3 / T6 and T4 / T5 fan out to disjoint crates.

## Task Dependency Graph

| ID | Title | Predecessors | HITL/AFK | Files owned (high level) |
|---|---|---|---|---|
| T1 | Strip dead code | none | AFK | Delete `crates/regions/src/specialization.rs`; delete `crates/regions/src/discovery.rs` `explorer_discovery_system` and `researcher_study_system` (keep `decomposer_discovery_system` for now — T4 universalises it) along with `StudyProgress` resource and `StudyComplete` message; delete `crates/ai/src/rival.rs`, `crates/ai/src/combat.rs`; delete `crates/ui/src/ability_bar.rs`, `crates/ui/src/spec_picker.rs`; delete `crates/input/src/specialization_input.rs`; remove `abilities.rs` from `crates/core/src/`; remove render-side rival code in `crates/render/src/data_layer.rs` (`RivalBranchGraph` resource, `extract_rival_branch_graph` system) and `crates/render/src/network_render.rs` (`rival_graph` parameter on `network_render_system`, `group_rival_nodes_by_id` helper); update `kingdom_ai`, `kingdom_regions`, `kingdom_ui`, `kingdom_input`, `kingdom_core`, `kingdom_render` `lib.rs` files; remove `RivalAiPlugin` from `bin/src/plugins.rs`; remove `Action::SetSpecializationTarget` and similar from `crates/input/src/action.rs`; delete corresponding tests. **Preserve** `crates/regions/src/mutation.rs` and `crates/ui/src/slot_machine_ui.rs` — they remain in the codebase as dormant infrastructure. Game compiles and plays the existing tip-based loop without specialization, rivals, or removed discovery paths. |
| T2 | Migrate data model | T1 | AFK | `crates/core/src/tile.rs` (drop `Occupant`, add `region_id`, add `radiation`, rename `nutrient_level` to `soil_richness`); `crates/core/src/region.rs` (strip spec fields, add `sugars`, `melanin`, rename `biomass` to `total_biomass`); `crates/core/src/constants.rs` (add new constants); migrate every call site across all crates from `Occupant::Player(rid)` and `is_player()` to `region_id` checks; update `crates/world/src/terrain_gen.rs` to seed `radiation`. Game compiles; tip-based loop still runs against new fields. |
| T3 | Replace growth core | T2 | AFK | Delete `crates/growth/src/tip.rs`, `crates/growth/src/decay.rs`, most of `crates/growth/src/nutrient.rs` (keep `nutrient_gradient_system`); add `crates/growth/src/density_flow.rs`, `crates/growth/src/dieback.rs`, `crates/growth/src/moisture.rs`, `crates/growth/src/bias_decay.rs`; update `crates/growth/src/lib.rs` plugin registration and chain order; modify `crates/world/src/region_tracking.rs` for new ownership semantics; modify `crates/render/src/data_layer.rs` (delete `extract_tip_positions`, `TipPositions`; update `extract_branch_graph`); modify `crates/render/src/entity_render.rs` (delete `tip_render_system`); update `crates/render/src/lib.rs`. Mycelium grows via density flow. |
| T4 | Resource systems (decomp + symbiosis + melanin) | T3 | AFK | Add `crates/growth/src/symbiosis.rs`, `crates/growth/src/melanin.rs`; modify `crates/regions/src/discovery.rs` to host the new universal `decomposition_system` (subsuming `decomposer_discovery_system`); modify `crates/regions/src/slot_machine.rs` publisher wiring; update `crates/growth/src/lib.rs` and `crates/regions/src/lib.rs` system registration. Sugars accumulate from decomp and symbiosis; melanin accumulates from radiation. |
| T5 | Wisp input + bias glow render | T3 | AFK | Delete `crates/input/src/priority.rs`; add `crates/input/src/wisp.rs`; update `crates/input/src/action.rs` (add `Action::Paint`, remove old priority actions); update `crates/input/src/lib.rs`; modify `crates/render/src/data_layer.rs` (`PriorityBiasMap` extraction unchanged; ensure compatible with glow renderer); replace `priority_arrow_render_system` in `crates/render/src/entity_render.rs` with `bias_glow_render_system`. Mouse drag paints bias; bias glow visible on map. |
| T6 | UI simplification | T2 | AFK | Modify `crates/ui/src/hud.rs` (drop specialization tier display, add `sugars` and `melanin` readouts); modify `crates/ui/src/tile_popover.rs` (drop specialization fields, show terrain, biomass, moisture, radiation, region_id, contents); update `crates/ui/src/lib.rs` plugin wiring. HUD reflects new resource model. |
| T7 | Integration tests + verification | T3, T4, T5, T6 | AFK | Add integration tests (`paint_then_grow`, `decompose_to_fragment_fusion`, `dry_zone_dieback`, `symbiosis_provides_sugars`, `radiation_provides_melanin`, `slot_machine_triggers_on_unique_decomp`, `path_split_no_rival_interference`) under appropriate `crates/*/tests/` directories. Verify fruiting recipes still progress correctly. Run full `just lint` and `just test` clean. |

Parallel batches:

```
Batch 1: T1
Batch 2: T2
Batch 3: T3 || T6        (different crates, no file overlap)
Batch 4: T4 || T5        (different crates, no file overlap; both depend on T3)
Batch 5: T7
```

## Agent Assignments

```
T1: Strip dead code                    → bevy-engineer  (Bevy/Rust)
T2: Migrate data model                 → bevy-engineer  (Bevy/Rust)
T3: Replace growth core (density flow) → bevy-engineer  (Bevy/Rust)
T4: Resource systems                   → bevy-engineer  (Bevy/Rust)
T5: Wisp input + bias glow render      → bevy-engineer  (Bevy/Rust)
T6: UI simplification                  → bevy-engineer  (Bevy/Rust)
T7: Integration tests + verification   → bevy-engineer  (Bevy/Rust)
Polish:                                → bevy-engineer  (uniformly Bevy/Rust diff)
```

All tasks use `bevy-engineer` per the agent selection guide's Bevy detection rule (workspace `Cargo.toml` lists `bevy = "0.18"`; every task touches Bevy ECS systems, plugins, schedules, or render pipelines).

## Out of scope

- **Cards.** Discussed in design conversation, not in this refactor.
- **Mutation reward effects.** Slot machine spins but grants nothing in MVP. Reward design is deferred.
- **Recipes for path-locked mushrooms.** Optional extension to `kingdom_fruiting`; not required for the loop.
- **Fail state.** `GamePhase::Defeat` remains unused. Drought is recoverable.
- **Bevy `WinitSettings::desktop_app()` for power saving.** Trivial follow-up, not required for the refactor.
- **Stroke replay or visualization.** No `Vec<Vec2>` sample buffer in `WispState`; single-resource state machine is enough for MVP.
- **Rival fungus AI.** Deleted; can return later as a per-region modifier on the flow rule.
- **Specialization.** Deleted; can return later as cards or per-region modifiers.
- **Right-click dissuasion and click-hold desire well.** Stroke-only MVP per input section.
