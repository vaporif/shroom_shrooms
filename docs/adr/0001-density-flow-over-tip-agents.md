# 0001. Replace tip-agent growth with density-flow growth

Date: 2026-05-07
Status: Accepted
Related: `docs/specs/2026-05-07-kingdom-density-flow-design.md`

## Context

The original simulation modelled mycelium growth as a population of `HyphalTip` entities, each hopping one hex per tick along a combination of nutrient gradient and player-set priority bias. Tile ownership was binary — a tip entering a tile set `Tile.biomass = 0.5` and assigned `Occupant::Player(region_id)`. Eight specialization types (Decomposer, Parasite, Symbiont, Infiltrator, Hunter, Transporter, Explorer, Researcher) modified tip behavior, region passives, and active abilities.

When designing the new wisp-driven painting interaction and the density-aware "single distributed mind" feel described in the design document, we had to choose how to express continuous-feeling growth in code. Two approaches were on the table:

- **A2 — extend the tip system.** Keep `HyphalTip` entities. Add dual-speed growth (tips along bias trail leap further and grow biomass faster), continuous biomass progression (start at 0.15, climb toward 1.0 with sustained presence), bias decay, biomass-driven rendering, and inter-tile biomass diffusion to smooth visible growth. Tips stay as an internal abstraction the player never sees directly.

- **A3 — replace tips with density-flow simulation.** Delete `HyphalTip` and `hyphal_tip_system`. Each tick, every tile with `biomass > MIN_FLOW_DENSITY` computes weighted outflows to its passable neighbors based on player bias, nutrient gradient, and noise; deltas apply in a second phase. The visible "growing front" is whichever frontier hex has the lowest biomass — emergent, not tracked.

A2 was strictly safer. It kept all eight specialization branches working without rewriting them; it kept the existing tip-related tests; it shipped in days rather than weeks. The visible behavior, with biomass-driven rendering and small inter-tile diffusion, would have been indistinguishable from A3 to a player. The user actually leaned toward A3 on conceptual grounds, but A3's price tag — rewriting eight specialization tip-modifiers, retesting the resource economy, and risking regressions in fruiting and fragments — was real.

The deciding move was the user's choice to drop the specialization layer entirely. The reasons for that choice are out of scope for this ADR; what matters here is that once specialization was gone, A3's cost collapsed. The eight tip-modifiers no longer existed. Tips' last structural defender — being the integration point that held all the specialization variety — was removed. What remained was an awkward intermediate layer between input and tile state, justified by nothing.

## Decision

Replace the tip-agent simulation with a density-flow simulation. Specifically:

- Delete `HyphalTip` component, `hyphal_tip_system`, `decay_system`, and the bulk of `nutrient.rs` (keeping `nutrient_gradient_system`).
- Add `density_flow_system` (two-phase: compute outflows, apply deltas), `dieback_system`, `moisture_diffusion_system`, and `bias_decay_system`.
- `Occupant` enum is dropped. Without rivals, ownership collapses to `region_id: Option<RegionId>` plus a `biomass >= CLAIM_THRESHOLD` check.
- Region tracking, fruiting, and fragments read the new ownership semantics. Specialization, abilities, slot-machine reward effects, mutations, combat, and rival AI are stripped (covered by the linked spec).

## Consequences

### What we gain

The "single distributed mind" framing becomes literal in code — no internal hidden agents stand between input and tile state. Anastomosis (two flows into the same tile) and dieback are symmetric operations on the same field, expressed as inverse cases of the flow rule, not as separate special-case events. Continuous biomass produces continuous strand thickness in rendering; the discrete-hop artifacts that A2 had to mitigate dissolve at the simulation layer rather than the render layer. The `Occupant` enum's loss removes a one-of-two type that was paying its keep only because rivals existed.

### What we give up

The flow rule is a multi-dimensional tunable (autonomous baseline, bias weight, gradient weight, noise, water cost) and finding the right values requires playtesting. The eight specialization branches are gone; if they return later, they have to re-express themselves as per-region modifiers on the flow rule rather than as imperative tip-behavior overrides. Existing tip and specialization tests are deleted and rewritten — roughly fifteen tests, mostly mechanical to replace. The migration touches every site that read `Occupant::Player(rid)` or called `is_player()`, which is wide but mechanical.

### What we are not committing to

This decision does not lock in a permanent absence of specialization. If gameplay layers on top of the wisp-bias loop need build-identity variety, the natural shape is per-region flow modifiers (e.g. a "decomposer" tag that doubles `SUGAR_FROM_DECOMP` for that region) — a parameter on the flow rule, not an entity-component layer above it. Cards, the original design-doc layer, are also out of scope for now and can return as either flow modifiers or per-tile bias generators.

## Alternatives considered

- **A1 — wisp UI swap only.** Replace P-key with drag-paint, leave growth alone. Rejected: doesn't deliver the felt experience of painted intent translating into directional growth. Paint becomes a re-skin of P-key.
- **A2 — extend tips.** Described above. Rejected once specialization was dropped, since A2's main advantage was specialization compatibility.
- **A3 — density flow.** Selected.

## References

- Spec: `docs/specs/2026-05-07-kingdom-density-flow-design.md`
- Original tip system: `crates/growth/src/tip.rs` (to be deleted)
- Original ownership model: `crates/core/src/tile.rs` `Occupant` enum (to be deleted)
