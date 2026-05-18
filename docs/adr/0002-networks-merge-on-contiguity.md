# 0002. Networks merge when their mycelium grows contiguous

Date: 2026-05-18
Status: Accepted
Related: `docs/specs/2026-05-18-civ-rework-phase1-design.md`

## Context

The Civ rework turns the single mycelial network into many networks, each
acting as a "city": a connected component of owned tiles identified by a
monotonic `RegionId`. Phase 1 introduces founder units that travel the map and
seed new networks at distant sites.

A founder can seed a network anywhere reachable. Nothing stops two of the
player's networks from growing toward each other until their biomass occupies
adjacent hexes and the two masses become one connected region. The design has
to answer: what happens then?

The codebase makes this question unavoidable. `region_tracking_system` already
recomputes connected components of owned tiles every tick. The moment two
networks' biomass touches, the tracker sees a single component. The only
question is how identity is resolved, not whether the situation arises.

Three models were on the table:

- **Stay separate (Civ borders).** Each network keeps its identity
  permanently. Tiles in the overlap belong to whichever network's founding site
  is nearest along the mycelium. Borders meet; cities never merge. This is the
  literal Civ model.

- **Merge into one.** Contiguous mycelium is one network. The older network
  absorbs the younger; the younger `RegionState` dissolves. This is closest to
  the existing connected-components tracker.

- **Stay separate, no overlap.** Networks stay distinct and a network's growth
  is forbidden from entering another network's territory. Cleanest borders, but
  growth bumping into an invisible wall around your own city reads as
  artificial.

The Civ-borders model is the genre-faithful choice and the one a player
familiar with Civ would expect. Its cost is real, though: it requires a
nearest-site territory-assignment pass layered on top of connected components,
a stable per-network identity that survives tiles changing hands every tick,
and rules for what happens when a network's founding site is cut off from its
own assigned territory. That is a second graph algorithm and a second source of
truth on top of the tracker the game already runs.

## Decision

Networks merge when their mycelium becomes contiguous.

When `region_tracking_system` finds a connected component containing tiles from
more than one region, the region with the lowest `RegionId` survives — region
ids are monotonically increasing, so the lowest id is the oldest network. The
whole component is relabelled to the survivor. Each absorbed region transfers
its `sugars` and `melanin` into the survivor (a merge pools resources; nothing
is destroyed) and its `RegionState` is removed. The survivor simply keeps its
id.

The inverse — a network severed by dieback into two components — is handled
symmetrically. Components are processed in a fixed order (sorted by their
lowest member hex), and each component takes the lowest `RegionId` among its
member tiles; the component that does not keep the surviving id becomes a new
region with an empty resource bank.

## Consequences

### What we gain

The region model stays close to the connected-components tracker the game
already runs every tick. No second territory-assignment algorithm, no
nearest-site border computation, no separate identity store that has to be
reconciled with the tracker. The merge is a small, deterministic rule layered
onto an existing pass.

Merging becomes a genuine strategic choice rather than a quirk. Keeping
networks apart yields many small cities, each its own capturable-hive economy;
growing them together yields one large network with a combined resource pool.
The player decides by choosing where to paint growth. "Wide versus consolidated"
is a real lever, not an accident.

It also matches the biology. Fungal hyphae fuse on contact (anastomosis); two
colonies of the same organism becoming one is what mycelium actually does.

### What we give up

Merging is surprising for a Civ-like. A player steeped in Civ expects cities to
keep their identity forever; here, growing two networks together erases one of
them and lowers the city count. The "many cities" fantasy is something the
player has to actively maintain by keeping networks apart, not a guarantee the
system upholds.

A founder's work can be partly undone by ordinary growth: found a network, then
later let the parent grow into it, and the founded network's separate identity
is gone. This is intended — the resources merge, so nothing is lost but the
count — but it has to be communicated, or it reads as a bug.

Determinism now matters in a place it did not before. The pre-rework tracker
picked the surviving id from `HashMap` iteration order, which was invisible
because regions had no identity. Now the surviving id is player-facing, so id
assignment must be order-independent. The lowest-`RegionId` merge rule decides
which region survives a merge, and the lowest-member-hex sort fixes the order
in which components are processed, so split pieces that allocate fresh ids do
so deterministically.

### What we are not committing to

This does not preclude a future "split off a sub-network" verb. The merge rule
governs what happens automatically on contiguity; deliberate player-driven
partition is a separate mechanic that could be added later (Phase 2's
connectivity work is the natural home for it). If playtesting shows players want
to protect a network's identity, an explicit "keep separate" marker on a region
is an additive change, not a reversal of this decision.

## Alternatives considered

- **Stay separate (Civ borders).** Rejected for its cost: a nearest-site
  territory algorithm and a separate per-network identity store on top of the
  existing connected-components tracker, plus the edge cases of a network's
  founding site being cut off from its assigned territory. The monotonic
  `RegionId` plus the deterministic merge/split rule already gives a stable,
  cheap identity layer; the Civ-borders model adds a second graph algorithm for
  no proportional gain. The user chose merge over this model directly.
- **Stay separate, no overlap.** Rejected: forbidding a network from growing
  into its own sibling's territory makes the player's own mycelium behave like
  it is hitting a wall, which is artificial and hard to read.
- **Merge into one.** Selected.

## References

- Spec: `docs/specs/2026-05-18-civ-rework-phase1-design.md`
- Region tracker: `crates/world/src/region_tracking.rs`
- Region model: `crates/core/src/region.rs`
