# Civ rework — Phase 1: networks, hives, founder units

Status: ready for review
Date: 2026-05-18
Scope: simulation core, world, a new units crate, input, rendering, UI

## Goal

Turn the single-network mycelium game into a Civ-shaped game with multiple
networks acting as cities and mobile units that expand the player's reach.
Phase 1 delivers the core loop end-to-end: a network grows, its mycelium
reaches an insect hive and captures it, the hive produces founder units, and a
founder walks across the map to found a new network.

Phase 1 is the first of three planned sub-projects. It deliberately stops short
of the connectivity-gated economy (Phase 2) and the combat unit roster
(Phase 3). Rival factions are a separate project after that. The win condition
is unchanged: cover the fragment tiles, fruit the required mushrooms.

The end state of Phase 1: the game still wins the same way, but the player now
manages several networks instead of one, and expansion is a deliberate act
performed by units rather than an automatic consequence of growth.

## Background and constraints

The game today runs one mycelial network. Growth is a continuous density-flow
field on an 80x60 pointy-top hex grid: each tile carries `biomass` that flows
to neighbours weighted by player-painted `priority_bias`, the nutrient
gradient, and noise. A tile is owned when `region_id.is_some() && biomass >=
CLAIM_THRESHOLD`. `RegionStates` holds a `HashMap<RegionId, RegionState>`;
`region_tracking_system` recomputes connected components of owned tiles each
tick and assigns region ids. Resources are per-region: `sugars`, `melanin`,
plus per-tile `moisture`. The player paints growth with a "wisp" — left-mouse
drag writes `priority_bias`; a left-mouse tap selects a tile. Simulation
systems run inside `SimulationSystems`, gated by a 1-second `TickTimer` with
pause and 2x/4x speeds. The win condition lives in `GameState::victory()`.

Four facts constrain this design:

The simulation is real-time-with-pause and stays that way. Units move in real
time and freeze on pause, exactly like the mycelium. Hexes are the spatial
substrate for both; a hex is a patch of ground a unit spends travel-time
crossing. No turn structure is introduced.

`region_tracking_system` already merges contiguous owned tiles into one
connected component, but it picks the surviving `RegionId` from
`HashMap`-iteration order — non-deterministic. Phase 1 needs deterministic
merge behaviour because a network's `RegionId` is now a named, persistent
identity that is player-facing.

The left mouse button is fully consumed by the wisp (drag paints, tap
selects). Adding unit selection and move orders requires freeing the bare left
click. The prior design avoided right-click for trackpad reasons, so the wisp
moves behind a held modifier key instead.

There is no units crate, no unit entity, and no pathfinding. These are new.

## Architecture

Phase 1 adds a unit layer on top of the existing simulation. The mycelium sim
is unchanged. A new `kingdom_units` crate owns hives, units, and founding. The
region model in `kingdom_core` is unchanged in shape; `region_tracking_system`
is reworked to assign region ids deterministically.

```
[Input — every frame, ungated]
  wisp modifier held ──> wisp_input_system ──> Tile.priority_bias   (unchanged behaviour)
  wisp modifier free ──> pointer_system    ──> SelectedUnit / move order / TileTapped
                         cursor_system     ──> window cursor icon swaps with wisp mode

[Simulation — SimulationSystems, tick-gated]
  density_flow, dieback, moisture, ...        (unchanged)
  region_tracking_system   ──> connected components + deterministic merge / split
  hive_capture_system      ──> Hive.captured_by  follows the underlying tile's region
  hive_production_system   ──> spends owner sugars, spawns Founder units at cap limit
  unit_upkeep_system       ──> drains owner-region sugars per living unit

[Unit movement — every frame, ungated, frozen on pause]
  unit_movement_system     ──> advances units along their hex path

[Founding — every frame, ungated]
  founding_system          ──> consumes a Founder on a valid site, creates a network
```

| Crate | Change |
|---|---|
| `kingdom_core` | `RegionState` and `RegionState::new` are unchanged. New components `Hive`, `Unit`, `UnitKind`, `UnitMovement`. New resource `SelectedUnit`. New constants. New messages `HiveCaptured`, `NetworkFounded`. |
| `kingdom_world` | `terrain_gen` places `HIVE_COUNT` hive entities. `region_tracking_system` reworked: deterministic merge / split, resource transfer on merge, unit re-parenting on merge. |
| `kingdom_units` (new) | Hive capture, hive production, unit upkeep, unit movement, hex pathfinding (`pathfinding` crate's A*), founding. `UnitsPlugin`. |
| `kingdom_input` | New `Action::WispMode` (held modifier) and `Action::FoundNetwork`. `wisp_input_system` gated behind `WispMode`. New `pointer_system` for unit select / move order / tile tap. New `cursor_system` swaps the window cursor with wisp mode. |
| `kingdom_render` | Unit sprites with interpolated between-hex position; hive sprites tinted by capture state; a selection ring on the selected unit. |
| `kingdom_ui` | HUD shows network count and unit count vs cap, plus aggregate sugars/melanin. Unit panel with a "Found Network" button when a founder is selected. Hive capture/production shown in the tile popover. |
| `bin` | Register `UnitsPlugin` in `KingdomPlugins`. |
| workspace `Cargo.toml` | Add `pathfinding = "4"` to `[workspace.dependencies]`. `members = ["crates/*", "bin"]` already globs the new `crates/units` crate in. |

## Data model

### `RegionState`

```rust
pub struct RegionState {
    pub region_id: RegionId,
    pub sugars: f32,
    pub melanin: f32,
    pub total_biomass: f32,
    pub tile_count: u32,
}
```

`RegionState`'s field set is unchanged, and `RegionState::new` is unchanged —
it keeps its existing `sugars: 10.0` default. A network is just a connected
component of owned tiles plus its monotonic `RegionId` — the `RegionId` is the
identity. There is no founding-hex stored on `RegionState`. The bare
`create_region()` stays unchanged; `region_tracking_system` uses it for split
pieces and `founding_system` uses it for new networks.

Every production caller of `create_region()` sets its starting resources
explicitly, so the `RegionState::new` default never reaches production: in
`terrain_gen`, `init_player_region` sets `sugars = 100.0`; `founding_system`
sets `sugars = FOUNDER_SEED_SUGARS`; and `region_tracking_system`'s split
branch sets `sugars` and `melanin` to 0.0 directly on the fresh region (a
severed chunk rebuilds its own economy). Because the default is a don't-care
for production, it is left at 10.0 — changing it would only churn the bare
`create_region()` tests in other crates for no benefit.

Merge precedence uses `RegionId` ordering: ids are monotonically increasing, so
the lowest id in a merged component is the oldest network and survives. No
separate age field is needed.

### `Hive`

```rust
#[derive(Component, Clone, Debug, Reflect)]
pub struct Hive {
    pub captured_by: Option<RegionId>,  // None = neutral; Some = owning network
    pub production: f32,                // 0.0..=1.0 progress toward the next founder
}
```

A hive is an entity `(GridPos, Hive)`, placed at world gen on a plain soil
tile. The tile underneath stays ordinary soil so the mycelium can grow onto it;
capture is detected by reading that tile's ownership. Hives are not a
`TileContents` variant — `TileContents` models decomposable substrate, which a
hive is not.

### `Unit`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum UnitKind {
    Founder,   // Phase 1 ships only this variant; Scout/Soldier/Builder arrive in later phases
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Unit {
    pub kind: UnitKind,
    pub owner: RegionId,   // the network that produced the unit; pays its upkeep
}

#[derive(Component, Clone, Debug, Reflect, Default)]
pub struct UnitMovement {
    pub path: Vec<Hex>,    // remaining hexes to traverse, in order; empty = idle
    pub edge_progress: f32, // 0.0..1.0 along the edge from GridPos to path[0]
}
```

A unit entity is `(GridPos, Unit, UnitMovement)`. `GridPos` is the hex the unit
currently occupies; while moving, its rendered world position is interpolated
toward `path[0]` by `edge_progress`.

### `SelectedUnit`

```rust
#[derive(Resource, Default)]
pub struct SelectedUnit(pub Option<Entity>);
```

Lives in `kingdom_core` next to the existing `SelectedRegion`.

### New constants (`crates/core/src/constants.rs`)

| Constant | Default | Used by |
|---|---|---|
| `HIVE_COUNT` | 6 | terrain_gen |
| `HIVE_PRODUCTION_SUGAR_COST` | 1.0 | hive_production (sugars drained per tick while producing) |
| `HIVE_PRODUCTION_RATE` | 0.05 | hive_production (progress per tick → ~20 ticks per founder) |
| `UNIT_UPKEEP_SUGAR` | 0.1 | unit_upkeep (sugars per living unit per tick) |
| `UNIT_CAP_BASE` | 2 | hive_production (cap when no hives captured) |
| `UNIT_CAP_PER_HIVE` | 2 | hive_production (extra cap per captured hive) |
| `MIN_FOUNDING_DISTANCE` | 6 | founding (minimum hex distance from any owned tile to a valid founding site) |
| `UNIT_SPEED_HEXES_PER_SEC` | 1.0 | unit_movement (speed at Normal; scales with SimulationSpeed) |
| `FOUNDER_SEED_BIOMASS` | 1.0 | founding (biomass placed on the founded hex) |
| `FOUNDER_SEED_SUGARS` | 10.0 | founding (starting sugars of a founded network) |

All tuning values; promote to a runtime config later if playtesting needs
hot-reload.

### New messages (`crates/core/src/messages.rs`)

```rust
#[derive(Message)]
pub struct HiveCaptured { pub hive_pos: Hex, pub region_id: RegionId }

#[derive(Message)]
pub struct NetworkFounded { pub region_id: RegionId, pub seed: Hex }
```

Existing messages are untouched.

## Systems

### `region_tracking_system` — reworked (kingdom_world)

Runs in `SimulationSystems`, as today. Still computes connected components of
owned tiles. The new responsibilities:

The component helper must change. Today `connected_components` tags each
component with a single `RegionId` — whichever owned tile the flood-fill
happened to start from. The rework needs the *full set* of distinct
`region_id`s present in each component, so it can pick the merge survivor and
know which regions were absorbed. The helper returns, per component, the
`Vec<Hex>` of tiles and the set of distinct `RegionId`s present among those
tiles' current `region_id` values.

Each tick the system does the following:

1. Compute connected components of owned tiles.
2. Sort the components deterministically by their lowest member hex (compare
   `Hex.x`, then `Hex.y`). This fixed order makes id assignment
   order-independent — nothing depends on `HashMap` iteration order.
3. Iterate the components in that sorted order. For each component:
   - `candidate = min(member RegionIds in the component)`.
   - If `candidate` has not been claimed yet this tick, the component keeps
     `candidate` and inherits that region's `RegionState`, resources included.
   - If `candidate` is already claimed (a split — the same id is the min of
     more than one component), allocate a fresh id with `create_region()`, then
     explicitly set that new region's `sugars` and `melanin` to 0.0. The empty
     bank comes from this explicit assignment, not from any constructor default:
     a severed chunk rebuilds its own economy.
   - If a kept component also contains other member ids (a merge), each other
     member region transfers its `sugars` and `melanin` into the survivor and
     then its `RegionState` is removed. The whole component is relabelled to the
     survivor id.
4. Relabel the `region_id` of every tile whose region changed.
5. Remove regions with zero owned tiles, as today.

The net effect: the merge survivor is the lowest `RegionId` in the merged
component — the oldest network. The split survivor is the component that sorts
first by lowest member hex; it keeps the id and its resources, while the other
split pieces become fresh regions with empty banks. The whole pass is fully
deterministic.

There is one edge case worth stating explicitly: if a region R2 is absorbed by
a merge *and* also has a severed chunk in the same tick, R2's resource bank
follows the merge survivor and the severed chunk becomes a fresh region with an
empty bank.

**Unit re-parenting on merge.** When the system removes a region because a
merge absorbed it, every `Unit` whose `owner` field equals the removed
`RegionId` must be reassigned to the merge survivor's id. `Unit` is a
`kingdom_core` component, so `region_tracking_system` queries `&mut Unit`
directly and re-parents using the same survivor mapping it already computed.
Without this, a founder produced by an absorbed network keeps a dangling
`owner`: it pays zero upkeep forever yet still counts against the unit cap.

### `hive_capture_system` (kingdom_units)

`SimulationSystems`, ordered after `region_tracking_system`. For each `Hive`,
read the tile at the hive's hex. If `tile.is_owned()`, set `captured_by =
Some(tile.region_id)`; otherwise set `captured_by = None`. This keeps capture
synced through merges and dieback automatically — the hive simply follows its
tile. On a `None -> Some` or owner-change transition, fire `HiveCaptured`.

### `hive_production_system` (kingdom_units)

`SimulationSystems`. For each captured hive whose owner region exists: if the
current living-unit count is below the cap and the owner region has `sugars >
0`, drain `HIVE_PRODUCTION_SUGAR_COST` sugars (clamped at 0) and add
`HIVE_PRODUCTION_RATE` to `production`. When `production >= 1.0`, reset it to
0.0 and spawn a `Founder` unit entity at the hive's hex with `owner =
captured_by`. If the cap is reached or the owner has no sugars, production
stalls and no sugars are drained.

Unit cap is global to the player: `UNIT_CAP_BASE + captured_hive_count *
UNIT_CAP_PER_HIVE`. More hives raises the ceiling, so expansion scales the
army.

Several hives can finish production in the same tick. The cap is re-checked per
spawn: the system tracks units spawned so far this tick and stops spawning once
the running total reaches the cap, so simultaneous completions cannot overshoot
it. A hive that finishes but is blocked by the cap keeps its `production` at
1.0 and spawns on a later tick when room frees up.

### `unit_upkeep_system` (kingdom_units)

`SimulationSystems`. Each tick, for every living unit, drain `UNIT_UPKEEP_SUGAR`
from the unit's `owner` region, clamped at 0. An idle stockpile of founders
quietly bleeds the network that made them — the soft economic pressure against
hoarding, on top of the hard cap. A merge does not orphan units: when
`region_tracking_system` absorbs a region it re-parents that region's units to
the merge survivor (see above), so an absorbed network's founders keep paying
upkeep to the survivor. If a unit's owner region nonetheless no longer exists
(dissolved with no survivor), its upkeep is skipped for Phase 1. No unit dies of
starvation in Phase 1; unit death belongs to the Phase 3 combat work.

### `unit_movement_system` (kingdom_units)

`Update`, ungated, runs every frame for smooth motion. Returns immediately if
`SimulationSpeed::Paused`. For each unit with a non-empty `path`:

```
speed_mult = match speed { Normal => 1, Fast => 2, Fastest => 4, Paused => 0 }
edge_progress += UNIT_SPEED_HEXES_PER_SEC * speed_mult * time.delta_secs()
while edge_progress >= 1.0 and path not empty:
    GridPos = path.remove(0)        // step onto the next hex
    edge_progress -= 1.0
if path empty:
    edge_progress = 0.0
```

Unit speed scales with the simulation speed so units and mycelium share one
clock.

### `founding_system` (kingdom_units)

`Update`, ungated. Reacts to the `FoundNetwork` action (or the HUD button)
while a `Founder` is selected. The founder must be idle (empty path) and
standing on a valid founding hex. The valid-site predicate has three checks:

- the tile is passable — reuse the existing `TerrainType::is_passable()` method
  (`crates/core/src/tile.rs:18`; passable = Soil/Root/Ruin/Surface). Citing the
  one method keeps pathfinding and founding agreeing on a single definition of
  passable.
- `tile.region_id.is_none()` (unclaimed).
- the hex distance to the nearest owned tile of *any* region is
  `>= MIN_FOUNDING_DISTANCE`. This prevents founding adjacent to existing
  territory, which would trigger an instant merge.

On a valid found: despawn the founder entity; `let rid =
region_states.create_region();` then set that region's `sugars =
FOUNDER_SEED_SUGARS`; set the founded tile's `region_id` to `Some(rid)` and its
`biomass` to `FOUNDER_SEED_BIOMASS` (above `CLAIM_THRESHOLD = 0.3`, so the tile
is owned and density flow begins spreading from it); fire `NetworkFounded`. The
existing density-flow and `region_tracking_system` then grow the network with
no further special-casing.

Because `founding_system` despawns the selected founder, it must also clear
`SelectedUnit` on a successful found. The render selection-ring and the unit
panel must tolerate a stale `Entity` in `SelectedUnit` — they already query the
entity and skip when the lookup fails, so a despawned founder simply drops the
ring on the next frame.

### System ordering

Within `SimulationSystems`: existing growth systems, then
`region_tracking_system`, then `hive_capture_system`, then
`hive_production_system`, then `unit_upkeep_system`. Capture must read
post-tracking region ids; production reads post-capture ownership.

`unit_movement_system`, `pointer_system`, `cursor_system`, `founding_system`,
and the wisp system run every frame in `Update`, ungated.

## Input

The bare left click is freed for units; the wisp moves behind a held modifier.

### Action map changes (`crates/input/src/action.rs`)

| Action | Binding | Status |
|---|---|---|
| `Action::WispMode` | a held modifier key (default a free key, e.g. `KeyE`) | NEW |
| `Action::FoundNetwork` | `KeyF` | NEW |
| `Action::Paint` | Left mouse button | unchanged binding |
| Camera, Zoom, pause, speed | unchanged | unchanged |

`Action::Paint` stays bound to the left mouse button. Disambiguation is in
code: `wisp_input_system` acts on `Paint` only while `WispMode` is held;
`pointer_system` acts on `Paint` only while `WispMode` is not held. Exactly one
of the two responds to any given click.

### `wisp_input_system` (modified)

Gains an early return: if `WispMode` is not pressed, set the wisp phase to
`Idle` and return. Otherwise the existing tap/drag state machine runs
unchanged. The tap branch still emits `TileTapped`, but tap-to-select is now
also reachable from `pointer_system` (see below) — keeping it in the wisp path
too is harmless and preserves wisp-mode tile inspection.

### `pointer_system` (new, kingdom_input)

Runs when `WispMode` is not held. On `Paint` just-pressed, resolve the cursor
to a hex:

- If a `Unit` entity occupies that hex, set `SelectedUnit` to it.
- Else if a unit is selected, compute a hex path (A* via the `pathfinding`
  crate) from the unit's `GridPos` to the clicked hex over passable tiles and
  write it into the unit's `UnitMovement.path`. An unreachable target leaves
  the unit idle.
- Else clear `SelectedUnit` and emit `TileTapped` (existing tile selection).

No right-click, no drag — single left clicks only, trackpad-friendly.

### `cursor_system` (new, kingdom_input)

Sets the primary window's cursor icon: a distinct "wisp" icon while `WispMode`
is held, the default pointer otherwise. This is the visible signal of which
mode the left click is in.

### Pause behaviour

`pointer_system`, `founding_system`, and the wisp run while paused — the player
selects units, queues move orders, and paints bias during a pause.
`unit_movement_system` does not advance while paused, so a queued path simply
waits. Tick-gated systems (capture, production, upkeep) do not run while paused.

## Rendering

All additions follow the existing two-layer pattern and the `entity_render`
sprite conventions. New work lives in `kingdom_render`.

**Units.** A spawn system mirrors `spawn_organism_sprites`, watching `Added<Unit>`.
A founder reuses the existing fauna sprite — a parasited insect is a fauna body
— tinted a sickly fungal green to read as "infected." Units render *much
smaller than a hex* (a fraction of the hex-scale organism sprite), so a unit
reads as a small body walking the terrain rather than a tile-filling blob. A
per-frame `unit_position_system` (PostUpdate) sets each unit sprite's
`Transform` by interpolating between `GridPos` and `path[0]` by `edge_progress`:
the small sprite physically travels hex-centre to hex-centre, visibly crossing
each hex it traverses. Units despawn cleanly when the `Unit` entity is removed
(reuse the `RemovedComponents` despawn pattern).

**Hives.** A hive sprite at each hive hex, drawn beneath units. Phase 1 reuses
an existing sprite (the neutral-fungus sprite is a reasonable stand-in) tinted
grey while neutral and the owner colour once captured; dedicated hive art is a
later polish item.

**Selected unit.** A ring sprite around `SelectedUnit`, following the unit each
frame; cleared when selection clears. The ring system queries the selected
`Entity` and skips when the lookup fails, so a founder despawned by founding
drops the ring cleanly without a dangling reference.

## UI

`kingdom_ui` changes, all in `hud.rs` and `tile_popover.rs`.

**HUD.** Add a network count and a unit count against the cap, alongside the
existing readouts. Sugars and melanin become aggregate totals summed across all
networks, giving an at-a-glance economy:

```
[Sugars: 41]  [Melanin: 3]  [Networks: 2]  [Units: 1/4]  [Turn: 142]  [>> 2x]  [Fragments: 2/5]  [Mushrooms: 0/3]
```

**Unit panel.** When `SelectedUnit` is set, a small panel shows the unit kind
and a "Found Network" button. The button is enabled only when the selected
founder is idle and standing on a valid founding hex (the same predicate
`founding_system` checks); clicking it triggers the found. The `KeyF` binding
does the same thing for keyboard players.

**Hive info.** When a tile with a hive on it is selected, the tile popover
shows the hive's capture state and production progress.

## Gameplay loop

```
start with one network (seeded at map centre), six hives scattered on the map
   |
paint the wisp (hold wisp modifier, drag) -> mycelium flows toward a hive
   |
mycelium biomass covers the hive's hex -> hive_capture_system claims it
   |
captured hive spends the network's sugars -> produces a founder unit (capped)
   |
left-click the founder to select it, left-click a distant valid spot to move it
   |
founder walks the hex path in real time (pause to redirect)
   |
founder reaches a valid site (passable, unclaimed, >= MIN_FOUNDING_DISTANCE
from any owned tile) -> "Found Network" -> founder consumed, new network seeded
   |
new network grows via density flow, reaches more hives -> loop
   |
two networks grown contiguous -> region_tracking merges them, pools resources
   |
cover the fragment tiles, fruit the mushrooms -> Victory (unchanged)
```

The win condition is untouched. The Civ layer is the means: more networks and
more hives mean more founders and faster map coverage, but every founder costs
sugars that fruiting also needs — the wide-versus-tall tension.

## Decisions and trade-offs

### Networks merge when contiguous

**Pros:** Keeps the region model close to today's connected-components
algorithm — low refactor cost. Gives the player a real choice: keep networks
apart for many small cities, or grow them together into one giant. Matches
fungal biology (hyphae fuse).

**Cons:** Surprising for a Civ-like — Civ cities never merge. Expansion can
erase your own city count. Merge precedence and resource transfer need explicit,
deterministic handling.

**Why we chose this:** The user chose it over Civ-style fixed borders. It is
also the cheapest model to build on top of the existing `region_tracking_system`,
and the "grow them together or keep them apart" decision is genuine strategic
texture rather than a quirk. See ADR 0002.

### A new `kingdom_units` crate rather than extending `kingdom_ai`

**Pros:** Matches the one-domain-per-crate workspace convention. Units, hives,
pathfinding, and founding are a coherent domain with no overlap with the
organism AI. Keeps `kingdom_ai` focused on wildlife.

**Cons:** One more crate, one more plugin to register, marginally longer build
graph.

**Why we chose this:** The workspace is already organised by domain crate;
folding units into `kingdom_ai` would blur that boundary for no benefit.

### Wisp behind a held modifier, units on the bare left click

**Pros:** Zero ambiguity — exactly one system responds to a click. Units, the
foreground Civ interaction, get the default mouse. No right-click, so trackpad
support is preserved. The cursor swap makes the active mode visible.

**Cons:** Painting growth now needs a key held — a small extra demand on the
player, and a two-handed gesture on a laptop. The wisp, formerly the primary
verb, becomes a moded one.

**Why we chose this:** Units must be directly clickable for a Civ-style game,
and the prior design ruled out right-click. Moding the less-frequent verb
(deliberate painting) rather than the constant one (unit control) is the right
trade. The user specified the modifier and cursor swap directly.

### Founders cost sugars to produce, sugars per tick to keep, and are hard-capped

**Pros:** Expansion becomes an economic decision tied to the same resource as
fruiting — the win condition — so the Civ layer is wired into victory rather
than bolted beside it. The per-tick upkeep makes idle stockpiles actively
costly; the hard cap stops stockpiling outright. Self-balancing: a poor network
cannot expand.

**Cons:** Three knobs (production cost, upkeep, cap) to tune. A network at zero
sugars stalls completely, which can feel punishing without clear UI feedback.

**Why we chose this:** The user asked for both a cap and sugar consumption.
Tying expansion to sugars produces the wide-versus-tall tension that is the
core of Civ strategy.

### Unit movement is frame-based and real-time, not tick-gated

**Pros:** Smooth, continuously animated motion. Units and mycelium share one
clock (speed scales together; pause freezes both). Matches the real-time-with-
pause decision.

**Cons:** Movement is decoupled from the tick, so a unit can finish a move
between ticks — slightly more state to reason about than a tick-quantised step.

**Why we chose this:** Tick-quantised movement would make units teleport one
hex per second, which reads as turn-based and fights the real-time identity.

## Testing strategy

The existing test pattern carries forward — `MinimalPlugins`, `test_app()`
helpers, `spawn_tile`-style fixtures, `cargo nextest run -p <crate>`. No new
infrastructure.

### Unit tests

- `region_tracking_system`: contiguous regions merge to the lowest `RegionId`;
  an absorbed region's sugars transfer to the survivor; a severed split piece
  that does not keep the surviving id gets a fresh region with an empty bank;
  units owned by an absorbed region are re-parented to the merge survivor; an
  empty region is removed.
- `hive_capture_system`: a hive on an owned tile is captured by that region; a
  hive on an unowned tile is neutral; capture follows the tile through a merge;
  `HiveCaptured` fires on the transition.
- `hive_production_system`: a captured hive with sugars accumulates production
  and spawns a founder at 1.0; production stalls at the unit cap; production
  stalls at zero sugars; the cap rises with captured hive count.
- `unit_upkeep_system`: each living unit drains its owner's sugars; sugars clamp
  at zero; a unit with a missing owner region is skipped.
- `unit_movement_system`: a unit advances along its path; it does not advance
  while paused; speed scales with `SimulationSpeed`; an emptied path leaves the
  unit idle on its final hex.
- pathfinding: the `find_path` adapter (A* via the `pathfinding` crate) finds a
  path over passable tiles; routes around impassable terrain; returns nothing
  for an unreachable target.
- `founding_system`: a founder on a valid site creates a region with the seeded
  tile owned and seeded biomass; founding is rejected within
  `MIN_FOUNDING_DISTANCE` of any owned tile; founding is rejected on a claimed
  or impassable tile; the founder entity is consumed and `SelectedUnit` cleared
  on success.
- `pointer_system`: a click on a unit selects it; a click with a unit selected
  issues a move order; a click on empty ground with no selection emits
  `TileTapped`; the system is inert while `WispMode` is held.

### Integration tests

| Test | What it proves |
|---|---|
| `grow_to_capture_hive` | Paint toward a hive, run ticks → `Hive.captured_by` is set, `HiveCaptured` fired. |
| `captured_hive_produces_founder` | Captured hive + sugars, run ticks → a `Founder` unit entity exists, owner sugars dropped. |
| `founder_walks_and_founds_network` | Move a founder to a valid site, found → a new region exists with the seeded tile owned and growing biomass; founder consumed. |
| `two_networks_merge_pools_resources` | Grow two networks contiguous → one region remains with the lower id and the summed sugars. |
| `unit_cap_blocks_overproduction` | Hold at the unit cap, run ticks → no founder beyond the cap, no sugars drained. |
| `upkeep_drains_idle_units` | Spawn founders, idle them, run ticks → owner sugars fall by upkeep. |
| `wisp_mode_gates_painting` | Left-drag with `WispMode` released paints nothing; with it held, paints bias. |

## Execution Strategy

**Subagents.** Every task runs in a fresh dispatched agent. Phase 1 is a single
tightly-coupled vertical stack: each task extends the network/hive/unit model
the previous one established, and the tasks share `kingdom_core`, the new
`kingdom_units` crate, and `kingdom_render`. There is no honest parallelism, so
the executor dispatches the six tasks strictly in sequence, each after its
predecessor's review passes.

## Task Dependency Graph

Each task is a thin vertical slice — schema, logic, render, and tests — and
leaves the game playable and demoable. All tasks are `AFK`: the design pins down
the behaviour, so no human checkpoint is required mid-implementation.

| ID | Title | Predecessors | HITL/AFK | Files owned (high level) |
|---|---|---|---|---|
| T1 | Network identity & deterministic tracking | none | AFK | `crates/world/src/region_tracking.rs` (rework `connected_components` to return the full member-id set per component; deterministic merge / split sorted by lowest member hex, resource transfer, the split branch sets the fresh region's `sugars`/`melanin` to 0.0 explicitly, unit re-parenting on merge); `crates/world/src/terrain_gen.rs` (keep `init_player_region`'s explicit starting sugars); `crates/ui/src/hud.rs` (show network count). `RegionState` and `RegionState::new` are untouched. Game still plays as one network. |
| T2 | Hives as map features | T1 | AFK | `crates/core/src/components.rs` (add `Hive`); `crates/core/src/messages.rs` (add `HiveCaptured`); `crates/core/src/constants.rs` (add `HIVE_COUNT`); `crates/world/src/terrain_gen.rs` (place `HIVE_COUNT` hive entities on the soil pool, clear of the player start); create the `kingdom_units` crate with `UnitsPlugin` and `hive_capture_system`; register in `bin/src/plugins.rs` and add `pathfinding` to the workspace `Cargo.toml` (`members = ["crates/*", "bin"]` globs the new crate in automatically); `crates/render/src/entity_render.rs` + `assets.rs` (hive sprites tinted by capture state); `crates/ui/src/tile_popover.rs` (hive capture state). Grow onto a hive → it shows captured. |
| T3 | Founder production + units | T2 | AFK | `crates/core/src/components.rs` (add `Unit`, `UnitKind`, `UnitMovement`); `crates/core/src/constants.rs` (add production/upkeep/cap constants); `crates/units/src` (`hive_production_system`, `unit_upkeep_system`); `crates/render/src/entity_render.rs` (founder unit sprites, static position for now); `crates/ui/src/hud.rs` (unit count vs cap, aggregate resources). Captured hive produces founders; sugars drain. |
| T4 | Unit movement, control, wisp rebind | T3 | AFK | `crates/units/src` (`unit_movement_system`, hex pathfinding via the `pathfinding` crate's `astar` — `grid.neighbors` as the successor function, `Hex::unsigned_distance_to` as the heuristic); `crates/units/Cargo.toml` (add `pathfinding` dep); `crates/core/src/components.rs` (add `SelectedUnit` resource, next to `SelectedRegion`); `crates/input/src/action.rs` (`WispMode`, `FoundNetwork`); `crates/input/src/wisp.rs` (gate behind `WispMode`); `crates/input/src/selection.rs` or new `pointer.rs` (`pointer_system`); `crates/input/src` (`cursor_system`); `crates/render/src/entity_render.rs` (interpolated unit position, selection ring). Select a founder, click to move it; wisp needs the modifier. |
| T5 | Founding new networks | T4 | AFK | `crates/core/src/messages.rs` (add `NetworkFounded`); `crates/core/src/constants.rs` (add `MIN_FOUNDING_DISTANCE`, `FOUNDER_SEED_*`); `crates/units/src` (`founding_system`, valid-site predicate); `crates/ui/src/hud.rs` (unit panel + "Found Network" button). Walk a founder to a valid spot, found → a new network appears and grows. |
| T6 | Integration tests + verification | T5 | AFK | Integration tests under the relevant `crates/*/tests/` directories (the seven listed in Testing strategy). Verify fruiting still wins. Run `just lint` and `just test` clean. |

Parallel batches: none. The executor dispatches T1 → T2 → T3 → T4 → T5 → T6,
each after the previous task's review passes.

## Agent Assignments

```
T1: Network identity & tracking     → bevy-engineer  (Bevy/Rust)
T2: Hives as map features           → bevy-engineer  (Bevy/Rust)
T3: Founder production + units      → bevy-engineer  (Bevy/Rust)
T4: Unit movement, control, wisp    → bevy-engineer  (Bevy/Rust)
T5: Founding new networks           → bevy-engineer  (Bevy/Rust)
T6: Integration tests + verification → bevy-engineer  (Bevy/Rust)
Polish:                             → bevy-engineer  (uniformly Bevy/Rust diff)
```

All tasks use `bevy-engineer` per the agent selection guide's Bevy detection
rule: the workspace `Cargo.toml` lists `bevy = "0.18"`, and every task touches
Bevy ECS systems, plugins, components, resources, or render code.

## Out of scope

- **Connectivity-gated economy.** Phase 2. In Phase 1 each network has its own
  independent pool; merging combines pools, but no sharing across separate
  networks.
- **Builders and rhizomorph cords.** Phase 2.
- **Scout and Soldier units, combat, hive production choices.** Phase 3. Phase 1
  ships only the `Founder` unit kind.
- **Rival fungal factions.** A separate project after Phase 3.
- **Unit death and starvation.** No unit dies in Phase 1; upkeep only drains
  sugars. Death arrives with Phase 3 combat.
- **A new win condition.** Fragments fused plus mushrooms fruited is unchanged.
- **Dedicated hive art.** Phase 1 reuses existing sprites tinted; bespoke art
  is a later polish pass.
- **Multi-unit box selection.** Single-unit selection only in Phase 1.
