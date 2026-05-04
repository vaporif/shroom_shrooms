# Slim core/ and Tick-Gate Render Extracts — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Move single-domain resources and messages out of `core/` into the crates that own them, and gate the simulation-driven render extract systems on `SimulationSet` so they stop re-scanning tiles every frame.

**Architecture:** Each domain crate registers the resources and messages only it consumes. `core/` keeps the cross-cutting state (`GridWorld`, `RegionStates`, `Tile`, `GameState`, `TickTimer`, `SimulationSet`, `GridPos`, `HexLayout`, `SelectedRegion`, `TurnAdvanced`, etc.). Render extract systems whose source data only changes on tick run inside `SimulationSet`; the two input-driven extracts (`extract_priority_bias_map`, `extract_selected_region_tiles`) stay in plain `Update` so cursor feedback does not lag.

**Tech Stack:** Rust 2024 edition, Bevy 0.18 (Messages, SystemSets, Plugins).

## Execution Strategy

**Subagents** — default, no spec override. Each task is a small, bounded refactor with clear file ownership and a green-build verification at the end. Sequential execution avoids merge conflicts on `crates/core/src/lib.rs`, which every move task edits.

## Task Dependency Graph

- Task 1 [AFK]: depends on `none` → first batch
- Task 2 [AFK]: depends on `Task 1` (shared edit on `crates/core/src/lib.rs`)
- Task 3 [AFK]: depends on `Task 2`
- Task 4 [AFK]: depends on `Task 3`
- Task 5 [AFK]: depends on `Task 4`
- Task 6 [AFK]: depends on `Task 5`
- Task 7 [AFK]: depends on `Task 6`
- Task 8 [AFK]: depends on `none` (touches only `crates/render/src/lib.rs`; can run any time)
- Polish: depends on `Task 1, …, Task 8`

Sequential ordering for moves is the safe default. Task 8 (tick-gate) is fully independent and can run in parallel with any move task if a runner supports it; otherwise queue it last before Polish.

## Agent Assignments

- Task 1: Move `MutationSelection` to `regions/` → `rust-engineer`
- Task 2: Move `SporeAction` to `fruiting/` → `rust-engineer`
- Task 3: Move `ActiveAbilityEffects` to `ui/` → `rust-engineer`
- Task 4: Move `HintsVisible` to `ui/` → `rust-engineer`
- Task 5: Move `TerrainSpriteMap` to `render/` → `rust-engineer`
- Task 6: Move `SlotMachineTriggered` message to `regions/` → `rust-engineer`
- Task 7: Move `NeutralFungiMerged` message to `ai/` → `rust-engineer`
- Task 8: Tick-gate render extracts → `rust-engineer`
- Polish: post-implementation-polish → `general-purpose`

---

## Background — verified facts

The decisions below rest on these grep findings (validator confirmed 2026-05-04). If any of these have changed since the plan was written, stop and re-verify before editing.

- `MutationSelection` (`core/abilities.rs:29`) is read/written only by `regions/` and `ui/`. Production init is **only** in `core/lib.rs:32`; the references at `regions/mutation.rs:34,59` are inside `#[cfg(test)] mod tests` and do not register the resource at runtime. Task 1 must add a fresh `.init_resource::<MutationSelection>()` to `UnlockPlugin::build` (which already chains `mutation_system`).
- `SporeAction` (`core/abilities.rs:34`) is used only by `fruiting/` and `ui/`. Production init is **only** in `core/lib.rs:33`; the reference at `fruiting/spores.rs:106` is inside `#[cfg(test)] mod tests`. Task 2 must add a fresh `.init_resource::<SporeAction>()` to `FruitingPlugin::build`.
- `ActiveAbilityEffects` (`core/abilities.rs:51`) is touched only by `ui/ability_bar.rs`. Pure UI state.
- `HintsVisible` (`core/simulation.rs:121`) is touched only by `ui/hud.rs:136`.
- `TerrainSpriteMap` (`core/simulation.rs:116`) is used only inside `render/terrain_render.rs`.
- `SlotMachineTriggered` (`core/messages.rs:36`) is written/read inside `regions/`; `ui/` only listens.
- `NeutralFungiMerged` (`core/messages.rs:42`) is written by `ai/organisms.rs`; nothing else reads it.
- `SelectedRegion` (3 readers: input/render/ui) **stays in core/**. It is genuinely shared.
- The render extract systems at `crates/render/src/lib.rs:31-42` run in plain `Update` with no `SimulationSet` gating. Five of the seven read tick-driven state and should move into `SimulationSet`; two read input-driven state and must stay every-frame.
- `crates/ui/Cargo.toml` currently depends only on `fungai_core` and `fungai_input`. Tasks 1, 2, and 6 each move a type that `ui/` consumes (`MutationSelection`, `SporeAction`, `SlotMachineTriggered`) into a crate `ui/` does not yet depend on. Each of those tasks must add the new workspace edge to `crates/ui/Cargo.toml`.

## Conventions for every move task

1. Define the type and its `Default` impl in the target crate (new file or appended to an existing one — see each task).
2. Register the resource (or message) in the target crate's `Plugin::build`. If the target plugin already self-inits the resource, that line stays and becomes the only one.
3. Remove the matching `.init_resource::<T>()` or `.add_message::<T>()` line from `crates/core/src/lib.rs`.
4. Remove the type definition from its old `core/` source file. Drop `pub use abilities::*;` style re-exports continue to work for types that remain in core; no edit needed there.
5. Update every `use fungai_core::T;` import across the workspace to point at the new crate.
6. Verify: `cargo check --workspace` clean, `cargo nextest run --workspace` green, `cargo clippy --workspace --all-targets -- -D warnings` clean.
7. Commit with a one-line message.

`grep -rn "fungai_core::.*\bT\b\|use fungai_core::\{[^}]*\bT\b" crates/` finds the consumers per type. Replace `T` with the moved type. Some types come in via `use fungai_core::*;` glob — those need no edit at the import line, but the glob is satisfied by the new crate's re-export only after Step 5 of the relevant task; check by compiling.

---

## Task 1: Move `MutationSelection` to `regions/`

**Files:**
- Edit: `crates/regions/src/mutation.rs` (host the moved struct; drop the now-self-import)
- Edit: `crates/regions/src/lib.rs` (register the resource in `UnlockPlugin::build`; re-export)
- Edit: `crates/core/src/abilities.rs` (remove struct)
- Edit: `crates/core/src/lib.rs` (remove `init_resource::<MutationSelection>()`)
- Edit: `crates/ui/Cargo.toml` (add `fungai_regions = { workspace = true }` — `ui/` does not yet depend on `regions/`)
- Edit: `crates/ui/src/slot_machine_ui.rs` (switch the `MutationSelection` import to `fungai_regions`)

- [x] **Step 1: Map current consumers**

Run: `grep -rn "MutationSelection" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: hits in `crates/core/src/abilities.rs`, `crates/core/src/lib.rs`, `crates/regions/src/mutation.rs` (use line + test-only inits), and `crates/ui/src/slot_machine_ui.rs`. Note every file path before editing.

- [x] **Step 2: Move the struct definition**

Open `crates/core/src/abilities.rs:29` and copy the `MutationSelection` struct (with its derives, plus any `impl Default` immediately following — `MutationSelection` derives `Default` so there is no separate impl). Paste it into `crates/regions/src/mutation.rs` near the top, below existing imports. `use bevy::prelude::*;` is already present at `mutation.rs:2`.

- [x] **Step 3: Drop the now-self-import in `mutation.rs`**

`crates/regions/src/mutation.rs:3` reads `use fungai_core::{MutationSelection, SlotMachineTriggered, UnlockOption};`. Remove `MutationSelection,` from that list (the type now lives in this crate). Leave `SlotMachineTriggered` and `UnlockOption` until later tasks move them.

- [x] **Step 4: Remove from core**

Delete the struct from `crates/core/src/abilities.rs`. Delete `.init_resource::<MutationSelection>()` from `crates/core/src/lib.rs:32`.

- [x] **Step 5: Add a production init in `UnlockPlugin`**

Open `crates/regions/src/lib.rs:60-71` (`UnlockPlugin::build`). Add `.init_resource::<MutationSelection>()` to the chain (it already inits `SlotMachineRng` and `AppliedMutations`). The existing `regions/mutation.rs:34,59` inits live inside `#[cfg(test)] mod tests` and do not register the resource for production — this new line is required.

- [x] **Step 6: Re-export from `regions/lib.rs`**

Update the existing `pub use mutation::{AppliedMutations, mutation_system};` at `crates/regions/src/lib.rs:16` to also re-export `MutationSelection`.

- [x] **Step 7: Add the workspace dep on `regions/` to `ui/`**

`crates/ui/Cargo.toml` lists only `fungai_core` and `fungai_input`. Add `fungai_regions = { workspace = true }` next to those entries.

- [x] **Step 8: Update consumer imports**

In `crates/ui/src/slot_machine_ui.rs:2`, change `use fungai_core::{MutationSelection, SlotMachineTriggered, UnlockOption};` so that `MutationSelection` is sourced from `fungai_regions`. Leave `SlotMachineTriggered` / `UnlockOption` on the `fungai_core` line until Task 6 moves them.

- [x] **Step 9: Verify build and tests**

Run: `cargo check --workspace`
Expected: clean.
Run: `cargo nextest run --workspace`
Expected: all tests pass.
Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [x] **Step 10: Commit**

Run: `git add -A && git commit -m "move MutationSelection from core to regions"`

---

## Task 2: Move `SporeAction` to `fruiting/`

**Files:**
- Edit: `crates/fruiting/src/spores.rs` (host the moved struct + `Default` impl; drop the now-self-import)
- Edit: `crates/fruiting/src/lib.rs` (register the resource in `FruitingPlugin::build`; re-export)
- Edit: `crates/core/src/abilities.rs` (remove `SporeAction` struct + `Default` impl at `:34-48`)
- Edit: `crates/core/src/lib.rs` (remove `init_resource::<SporeAction>()` at `:33`)
- Edit: `crates/ui/Cargo.toml` (add `fungai_fruiting = { workspace = true }` — `ui/` does not yet depend on `fruiting/`)
- Edit: `crates/ui/src/ability_bar.rs` (switch the `SporeAction` import to `fungai_fruiting`)

- [x] **Step 1: Map consumers**

Run: `grep -rn "SporeAction" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: hits in `crates/core/src/abilities.rs`, `crates/core/src/lib.rs`, `crates/fruiting/src/spores.rs` (use line + test-only inits/uses), and `crates/ui/src/ability_bar.rs`.

- [x] **Step 2: Move struct + `Default` impl**

Cut `pub struct SporeAction { ... }` and `impl Default for SporeAction { ... }` from `crates/core/src/abilities.rs:33-48` (verify the exact range before cutting). Paste into `crates/fruiting/src/spores.rs` near the top, below the existing imports.

- [x] **Step 3: Drop the now-self-import in `spores.rs`**

`crates/fruiting/src/spores.rs:2-5` reads `use fungai_core::{ ..., SPORE_RELAY_ACCURACY_RADIUS, SporeAction, Tile, };`. Remove `SporeAction,` from that list (the type now lives in this crate). Leave the other names alone.

- [x] **Step 4: Remove from core's plugin**

Delete `.init_resource::<SporeAction>()` from `crates/core/src/lib.rs:33`.

- [x] **Step 5: Add a production init in `FruitingPlugin`**

Open `crates/fruiting/src/lib.rs:14-28` (`FruitingPlugin::build`). It currently inits `SporeRng` only. Add `.init_resource::<SporeAction>()` to that chain. The existing `spores.rs:106` init lives inside `#[cfg(test)] mod tests` and does not register the resource for production — this new line is required.

- [x] **Step 6: Re-export from `fruiting/lib.rs`**

Update the existing `pub use spores::{SporeRng, spore_system};` at `crates/fruiting/src/lib.rs:11` to also re-export `SporeAction`.

- [x] **Step 7: Add the workspace dep on `fruiting/` to `ui/`**

`crates/ui/Cargo.toml` lists only `fungai_core` and `fungai_input` (and, after Task 1, `fungai_regions`). Add `fungai_fruiting = { workspace = true }`.

- [x] **Step 8: Update consumer imports**

In `crates/ui/src/ability_bar.rs:5`, peel `SporeAction` out of the `use fungai_core::{ ... }` block and add `use fungai_fruiting::SporeAction;`.

- [x] **Step 9: Verify build and tests**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean.

- [x] **Step 10: Commit**

Run: `git add -A && git commit -m "move SporeAction from core to fruiting"`

---

## Task 3: Move `ActiveAbilityEffects` to `ui/`

**Files:**
- Create or edit: `crates/ui/src/ability_bar.rs` (single consumer)
- Edit: `crates/core/src/abilities.rs` (remove `ActiveAbilityEffects` struct at `:51`)
- Edit: `crates/core/src/lib.rs` (remove `init_resource::<ActiveAbilityEffects>()` at `:34`)
- Edit: `crates/ui/src/lib.rs` (register the resource in `UiPlugin::build` and re-export)

- [x] **Step 1: Map consumers**

Run: `grep -rn "ActiveAbilityEffects" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: only `core/abilities.rs`, `core/lib.rs`, `ui/ability_bar.rs`. If anything else hits, stop and reassess.

- [x] **Step 2: Move the struct definition**

Cut `pub struct ActiveAbilityEffects { ... }` (it derives `Default`, so no separate impl) from `crates/core/src/abilities.rs:50-53`. Paste into `crates/ui/src/ability_bar.rs` near the top, below the existing imports. Then drop `ActiveAbilityEffects` from the `use fungai_core::{ ... };` block at `ability_bar.rs:3-6` — the type now lives in this same file. `AbilityEffectType` and `ActiveEffect` stay in `fungai_core` (they remain referenced by the moved struct via the still-present `fungai_core` import).

- [x] **Step 3: Register in `UiPlugin`**

Open `crates/ui/src/lib.rs`. `UiPlugin::build` (lines 87-99) currently only adds sub-plugins; pick `AbilityBarPlugin` (lines 28-41) as the natural home and add `.init_resource::<ActiveAbilityEffects>()` to its build chain. No new `use` line is needed inside `lib.rs` (the resource type is referenced only inside `ability_bar.rs`'s submodule). Add `pub use ability_bar::ActiveAbilityEffects;` to the existing `pub use ability_bar::{...};` block at `:9-12` for downstream visibility.

- [x] **Step 4: Remove from core**

Delete `.init_resource::<ActiveAbilityEffects>()` from `crates/core/src/lib.rs:34`. Confirm `crates/core/src/abilities.rs` no longer holds the struct.

- [x] **Step 5: Verify build and tests**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean.

- [x] **Step 6: Commit**

Run: `git add -A && git commit -m "move ActiveAbilityEffects from core to ui"`

---

## Task 4: Move `HintsVisible` to `ui/`

**Files:**
- Edit: `crates/ui/src/hud.rs` (single consumer at `:136`)
- Edit: `crates/core/src/simulation.rs` (remove struct at `:121-125`)
- Edit: `crates/core/src/lib.rs` (remove `init_resource::<HintsVisible>()` at `:36`)
- Edit: `crates/ui/src/lib.rs` (register resource and re-export)

- [x] **Step 1: Map consumers**

Run: `grep -rn "HintsVisible" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: `core/simulation.rs`, `core/lib.rs`, `ui/hud.rs`.

- [x] **Step 2: Move the struct + `Default` impl**

Cut `pub struct HintsVisible(pub bool);` and `impl Default for HintsVisible { fn default() -> Self { Self(true) } }` from `crates/core/src/simulation.rs:120-127` (read the file first to confirm the exact range). Paste into `crates/ui/src/hud.rs` near the top, below the existing imports. Then drop `HintsVisible` from the `use fungai_core::{ ... };` block at `hud.rs:3` — the type now lives in this same file.

- [x] **Step 3: Register in `HudPlugin`**

`crates/ui/src/lib.rs`: `HudPlugin::build` (lines 21-26) currently only registers systems. Add `.init_resource::<HintsVisible>()` to its chain. Add `pub use hud::HintsVisible;` to the existing `pub use hud::{...};` line at `:13` for downstream visibility (the only other reader is the same crate, but a re-export keeps the public surface tidy).

- [x] **Step 4: Remove from core's plugin**

Delete `.init_resource::<HintsVisible>()` from `crates/core/src/lib.rs:36`.

- [x] **Step 5: Verify build and tests**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean.

- [x] **Step 6: Commit**

Run: `git add -A && git commit -m "move HintsVisible from core to ui"`

---

## Task 5: Move `TerrainSpriteMap` to `render/`

**Files:**
- Edit: `crates/render/src/terrain_render.rs` (single consumer; struct will live here)
- Edit: `crates/core/src/simulation.rs` (remove struct at `:116`)
- Edit: `crates/core/src/lib.rs` (remove `init_resource::<TerrainSpriteMap>()` at `:35`)
- Edit: `crates/render/src/lib.rs` (register the resource)

- [ ] **Step 1: Map consumers**

Run: `grep -rn "TerrainSpriteMap" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: `core/simulation.rs`, `core/lib.rs`, `render/terrain_render.rs`.

- [ ] **Step 2: Move struct + `Default` impl**

Read `crates/core/src/simulation.rs:115-118` (`TerrainSpriteMap` derives `Default`, so there is no separate impl block). Also cut the `use std::collections::HashMap;` at `simulation.rs:1` if no other type in that file still needs it (check first). Paste the struct into `crates/render/src/terrain_render.rs` near the top, below the existing imports. Add a local `use std::collections::HashMap;` at the top of `terrain_render.rs` if not already present. The file's `use fungai_core::*;` glob at `:9` previously surfaced the type; after the move, the type is in scope locally so no further `use` line is required.

- [ ] **Step 3: Register in `RenderPlugin`**

`crates/render/src/lib.rs`: add `.init_resource::<terrain_render::TerrainSpriteMap>()` near the other `init_resource` calls (around `:23-30`). Decide whether to expose: add `pub use terrain_render::TerrainSpriteMap;` only if a downstream crate needs it (Step 1's grep tells you).

- [ ] **Step 4: Remove from core's plugin**

Delete `.init_resource::<TerrainSpriteMap>()` from `crates/core/src/lib.rs:35`.

- [ ] **Step 5: Verify build and tests**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean.

- [ ] **Step 6: Commit**

Run: `git add -A && git commit -m "move TerrainSpriteMap from core to render"`

---

## Task 6: Move `SlotMachineTriggered` message to `regions/`

**Files:**
- Edit: `crates/regions/src/slot_machine.rs` (host the `Message` struct; canonical home over `mutation.rs`)
- Edit: `crates/regions/src/slot_machine.rs` and `crates/regions/src/mutation.rs` and `crates/regions/src/discovery.rs` (drop `SlotMachineTriggered` from their `use fungai_core::{...}` lines once the type lives in this crate)
- Edit: `crates/core/src/messages.rs` (remove struct at `:36`)
- Edit: `crates/core/src/lib.rs` (remove `add_message::<SlotMachineTriggered>()` at `:43`)
- Edit: `crates/regions/src/lib.rs` (add `add_message::<SlotMachineTriggered>()` to `RegionsPlugin::build` and re-export)
- Edit: `crates/ui/Cargo.toml` (add `fungai_regions = { workspace = true }` if Task 1 has not already added it)
- Edit: `crates/ui/src/slot_machine_ui.rs` (switch the `SlotMachineTriggered` import to `fungai_regions`)

- [ ] **Step 1: Map consumers**

Run: `grep -rn "SlotMachineTriggered" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: `core/messages.rs:36`, `core/lib.rs:43`, several files in `regions/` (`slot_machine.rs`, `mutation.rs`, `discovery.rs`), and `ui/slot_machine_ui.rs`.

- [ ] **Step 2: Move the message struct**

Cut `#[derive(Message)] pub struct SlotMachineTriggered { ... }` from `crates/core/src/messages.rs:35-39`. Paste into `crates/regions/src/slot_machine.rs` near the top, below existing imports. The struct uses `UnlockPool` (still in `fungai_core`) and `Vec<UnlockOption>` (also still in `fungai_core`); the existing `use fungai_core::{..., UnlockOption, UnlockPool};` already covers these. Extend the existing `use bevy::ecs::message::{MessageReader, MessageWriter};` at `slot_machine.rs:1` to also import `Message` (the derive needs the trait in scope).

- [ ] **Step 3: Register in `RegionsPlugin`**

`crates/regions/src/lib.rs`: in `RegionsPlugin::build` (lines 81-102), add `app.add_message::<SlotMachineTriggered>();` (it is fine to register here — `RegionsPlugin` is composed once by the binary). Update the existing `pub use slot_machine::{SlotMachineRng, slot_machine_system};` at `:17` to also re-export `SlotMachineTriggered`.

- [ ] **Step 4: Drop the now-self-imports inside `regions/`**

Three regions files import `SlotMachineTriggered` from `fungai_core`:
- `crates/regions/src/slot_machine.rs:3`
- `crates/regions/src/mutation.rs:3` (after Task 1 this line still imports `SlotMachineTriggered, UnlockOption`)
- `crates/regions/src/discovery.rs:7`

Remove `SlotMachineTriggered` from each of those `use fungai_core::{ ... };` blocks. The type is now in scope via `crate::slot_machine::SlotMachineTriggered` (or via a `use crate::slot_machine::SlotMachineTriggered;` line at the top of each consumer if a bare name is preferred).

- [ ] **Step 5: Remove from core's plugin**

Delete `.add_message::<SlotMachineTriggered>()` from `crates/core/src/lib.rs:43`.

- [ ] **Step 6: Ensure `ui/` depends on `regions/`**

If Task 1 already added `fungai_regions = { workspace = true }` to `crates/ui/Cargo.toml`, skip this step. Otherwise add it now.

- [ ] **Step 7: Update the UI consumer**

In `crates/ui/src/slot_machine_ui.rs:2`, peel `SlotMachineTriggered` out of the `use fungai_core::{ ... };` line and import it from `fungai_regions` instead.

- [ ] **Step 8: Verify build and tests**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean.

- [ ] **Step 9: Commit**

Run: `git add -A && git commit -m "move SlotMachineTriggered from core to regions"`

---

## Task 7: Move `NeutralFungiMerged` message to `ai/`

**Files:**
- Edit: `crates/ai/src/organisms.rs` (host the struct; the file is the sole writer and consumes the type via `use fungai_core::*;`)
- Edit: `crates/core/src/messages.rs` (remove struct at `:41-45`)
- Edit: `crates/core/src/lib.rs` (remove `add_message::<NeutralFungiMerged>()` at `:44`)
- Edit: `crates/ai/src/lib.rs` (add `add_message::<NeutralFungiMerged>()` to `AiPlugin::build`)

- [ ] **Step 1: Map consumers**

Run: `grep -rn "NeutralFungiMerged" /Users/vaporif/Repos/fungai/crates/ --include="*.rs"`
Expected: only `core/messages.rs`, `core/lib.rs`, and `ai/organisms.rs`. If any other file shows up, the validator finding was incomplete — re-evaluate before continuing.

- [ ] **Step 2: Move the message struct**

Cut `#[derive(Message)] pub struct NeutralFungiMerged { ... }` from `crates/core/src/messages.rs:41-45`. Paste into `crates/ai/src/organisms.rs` near the top, below the existing `use` block. The struct uses `RegionId`, which is already in scope through `use fungai_core::*;` at `organisms.rs:3`. Extend the existing `use bevy::ecs::message::MessageWriter;` at `organisms.rs:1` to also import `Message` (the derive needs the trait in scope).

- [ ] **Step 3: Confirm the glob import still resolves**

`crates/ai/src/organisms.rs:3` is `use fungai_core::*;`. Once `NeutralFungiMerged` is removed from `fungai_core`, the glob no longer provides it, but the type now lives in this same file so it is in scope directly. No new `use` line is required. Spot-check by running `cargo check -p fungai_ai` after editing.

- [ ] **Step 4: Register in `AiPlugin`**

`crates/ai/src/lib.rs:70-92`: add `app.add_message::<NeutralFungiMerged>();` to `AiPlugin::build`. Step 1's grep should show no downstream listener; if so, no `pub use` is required (skip the re-export).

- [ ] **Step 5: Remove from core's plugin**

Delete `.add_message::<NeutralFungiMerged>()` from `crates/core/src/lib.rs:44`.

- [ ] **Step 6: Verify build and tests**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean.

- [ ] **Step 7: Commit**

Run: `git add -A && git commit -m "move NeutralFungiMerged from core to ai"`

---

## Task 8: Tick-gate the simulation-driven render extracts

**Files:**
- Edit: `crates/render/src/lib.rs:31-42` (the `add_systems(Update, ...)` block)

**Background — split rationale:**

| Extract system | Source data | Decision |
|---|---|---|
| `extract_branch_graph` | `tile.occupant`, `tile.biomass` (tick-driven) | gate on `SimulationSet` |
| `extract_tip_positions` | `HyphalTip` components (tick-driven) | gate on `SimulationSet` |
| `extract_region_hulls` | `tile.occupant` (tick-driven) | gate on `SimulationSet` |
| `extract_discovery_map` | `BranchGraph` (derived, tick-driven) | gate on `SimulationSet` |
| `extract_rival_branch_graph` | `tile.occupant` (tick-driven) | gate on `SimulationSet` |
| `extract_priority_bias_map` | `tile.priority_bias` (input-driven, every frame) | **leave in plain `Update`** |
| `extract_selected_region_tiles` | `SelectedRegion` resource (input-driven) | **leave in plain `Update`** |

Gating the input-driven two would make the priority-arrow overlay and selection highlight lag the cursor by up to one tick (1.0s on Normal speed). Both already have built-in `if new != old { update }` guards (`data_layer.rs:133-135, 291-293, 308-310`), so the per-frame iteration cost is bounded.

- [ ] **Step 1: Read current state**

Read `crates/render/src/lib.rs`. Confirm the `Update` block matches lines 31-42 of the plan and that `SimulationSet` is not currently imported.

- [ ] **Step 2: Import `SimulationSet`**

Add `use fungai_core::SimulationSet;` near the existing `use bevy::...` lines.

- [ ] **Step 3: Split the systems tuple**

Replace the existing `.add_systems(Update, ( ... ))` block (originally `crates/render/src/lib.rs:31-42`) with two `add_systems` calls: one gated by `SimulationSet`, one ungated.

```rust
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
    ),
)
```

- [ ] **Step 4: Verify build, tests, lint**

Run: `cargo check --workspace && cargo nextest run --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all clean. The existing render tests in `crates/render/src/data_layer.rs:331-518` add the extract systems directly (without `SimulationSet`), so they continue to exercise the systems and stay green.

- [ ] **Step 5: Add a regression test for the gate**

Append to the `mod tests` block at the end of `crates/render/src/data_layer.rs`. The test asserts that when the render plugin is loaded together with `SimulationSet` configured but no tick has expired yet, `BranchGraph` stays empty:

```rust
#[test]
fn extract_branch_graph_does_not_run_outside_simulation_set() {
    use bevy::app::ScheduleRunnerPlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    app.init_resource::<GridWorld>();
    app.init_resource::<RegionStates>();
    app.init_resource::<BranchGraph>();
    app.insert_resource(create_hex_layout());

    // Configure SimulationSet so it never runs (no run_if applied — set just exists).
    // Without TickTimer advancement, the gate effectively suppresses the system.
    app.configure_sets(Update, fungai_core::SimulationSet.run_if(|| false));
    app.add_systems(Update, extract_branch_graph.in_set(fungai_core::SimulationSet));

    // Spawn a tile that *would* populate BranchGraph if the system ran.
    let rid = app.world_mut().resource_mut::<RegionStates>().create_region();
    let pos = Hex::ZERO;
    let e = app
        .world_mut()
        .spawn((
            GridPos(pos),
            Tile {
                occupant: Occupant::Player(rid),
                biomass: 1.0,
                ..default()
            },
        ))
        .id();
    app.world_mut().resource_mut::<GridWorld>().tiles.insert(pos, e);

    app.update();

    let graph = app.world().resource::<BranchGraph>();
    assert!(graph.nodes.is_empty(), "system ran despite gate");
}
```

Run: `cargo nextest run -p fungai_render extract_branch_graph_does_not_run_outside_simulation_set`
Expected: PASS.

- [ ] **Step 6: Smoke-test the running game**

Run: `just dev`
Expected: game starts, terrain renders, mycelium network renders when growing, priority arrows respond to clicks immediately (no perceptible lag), selection highlight responds immediately. Pause for ~30 seconds of gameplay across Normal, Fast, Fastest speeds. Close cleanly.

If priority arrows OR selection highlight appear laggy, Step 3's split was wrong — verify the two input-driven extracts are in the ungated block.

- [ ] **Step 7: Commit**

Run: `git add -A && git commit -m "gate simulation-driven render extracts on SimulationSet"`

---

## Polish

- [ ] **Step 1: Run post-implementation-polish skill**

Dispatch `post-implementation-polish` against this branch's diff. Expected scope: review rounds, idiomatic pass, `/cleanup`, AI-comment strip. Apply suggestions that match the project's style; defer any large refactor to a separate plan.

- [ ] **Step 2: Final verification**

Run: `just lint && just test`
Expected: all green.

- [ ] **Step 3: Confirm `core/lib.rs` is leaner**

Run: `git diff main -- crates/core/src/lib.rs`
Expected: 5 fewer `init_resource` calls (`MutationSelection`, `SporeAction`, `ActiveAbilityEffects`, `HintsVisible`, `TerrainSpriteMap`), 2 fewer `add_message` calls (`SlotMachineTriggered`, `NeutralFungiMerged`). `CorePlugin::build` should now contain only the cross-cutting state.

- [ ] **Step 4: Update CLAUDE.md if structure descriptions drifted**

Open `CLAUDE.md`. Current text describes the workspace at a high level — no per-resource detail to update. Confirm no edits needed; if something does need updating, do it now.
