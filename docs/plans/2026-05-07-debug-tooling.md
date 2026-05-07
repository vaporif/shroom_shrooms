# Debug Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Add four debugging tools — process CPU/RAM diagnostics, Tracy profiling feature, schedule graph export, runtime entity inspector — so we can pinpoint perf regressions instead of guessing.

**Architecture:** All four hang off the existing `DebugPlugin` in `bin/src/debug.rs` or the `bin/Cargo.toml` feature/dependency surface. Three are runtime-cheap and always loaded; Tracy is feature-gated so it doesn't pollute normal builds. The egui inspector is loaded but its UI is hidden until Ctrl+D.

**Tech Stack:** Bevy 0.18 (workspace pin), `bevy_mod_debugdump` (DOT schedule export), `bevy-inspector-egui` + `bevy_egui` (runtime inspector), Tracy via Bevy's built-in `trace_tracy` feature.

## Execution Strategy

**Subagents** — default; no spec override. All four tasks touch `bin/Cargo.toml` and several share `bin/src/debug.rs`, so they cannot safely run in parallel. Sequential subagent dispatch keeps Cargo.toml merge-clean.

## Task Dependency Graph

- Task 1 [AFK]: SystemInformationDiagnosticsPlugin — depends on `none`
- Task 2 [AFK]: Tracy feature flag — depends on `Task 1`
- Task 3 [AFK]: bevy_mod_debugdump CLI flag — depends on `Task 2`
- Task 4 [AFK]: bevy-inspector-egui F12 toggle — depends on `Task 3`
- Polish: post-implementation-polish — depends on `Task 1, 2, 3, 4`

Each task touches `bin/Cargo.toml`; running them serially avoids trivial Cargo.toml merge conflicts. Tasks 1 and 4 also share `bin/src/debug.rs`. Sequential dispatch is the clean path here — there's no real parallelism to extract from four small tasks all rooted in the same crate.

## Agent Assignments

- Task 1: SystemInformationDiagnosticsPlugin → bevy-engineer (Rust/Bevy)
- Task 2: Tracy feature flag → bevy-engineer (Rust/Bevy)
- Task 3: bevy_mod_debugdump CLI flag → bevy-engineer (Rust/Bevy)
- Task 4: bevy-inspector-egui F12 toggle → bevy-engineer (Rust/Bevy)
- Polish: post-implementation-polish → general-purpose

---

## Background

The recent zoom-and-frame-perf plan exposed a pre-existing bacteria spawn loop only after the frame-side CPU dropped enough for the leak to dominate. We had to instrument from scratch (per-archetype entity counts in `debug.rs`) before we could find the bug. This plan installs durable tooling so the next regression doesn't take that long to localise:

| Tool | Answers |
|---|---|
| `SystemInformationDiagnosticsPlugin` | "What's the process CPU/RAM right now?" — confirms whether we're CPU-bound or GPU-bound. |
| Tracy (via `bevy/trace_tracy` feature) | "Which system runs on which thread when?" — the gold-standard frame timeline. |
| `bevy_mod_debugdump` | "What's the static schedule dependency graph?" — DOT export of which systems block which. |
| `bevy-inspector-egui` | "What's in this entity right now?" — runtime entity/component/resource inspector. |

---

### Task 1: SystemInformationDiagnosticsPlugin

**Files:**
- Modify: `bin/src/debug.rs:1-45`

**Design choice — built-in plugin vs custom sysinfo polling:**

- **Built-in (selected):** Bevy ships `SystemInformationDiagnosticsPlugin` behind the `bevy/sysinfo_plugin` feature. It updates `SystemInformation::CPU_USAGE` and `SystemInformation::MEM_USAGE` on a low-frequency timer (~1 Hz) and writes them to `DiagnosticsStore` like our existing FPS/entity diagnostics. Auto-selected — no downsides compared to alternatives. Let me know if you disagree.
- Custom sysinfo polling: more code, no benefit.

The plugin needs the `bevy/sysinfo_plugin` feature flag. The workspace `bevy = "0.18"` pin already includes it via `default-features` in most Bevy distributions, but verify with `cargo tree` and add it explicitly if not.

- [x] **Step 1: Verify `sysinfo_plugin` feature is enabled in the bevy dep**

Run:
```
cargo tree -p kingdom -i bevy --edges features 2>&1 | grep -i sysinfo
```
Expected: a line referencing `sysinfo_plugin` if the workspace already enables it.

If it's NOT enabled, add it to the root `Cargo.toml` in the `[workspace.dependencies]` block (find the existing `bevy = "0.18"` line and convert to a table):

```toml
[workspace.dependencies]
bevy = { version = "0.18", features = ["sysinfo_plugin"] }
```

If it IS already enabled, skip the Cargo.toml edit.

- [x] **Step 2: Add the plugin to DebugPlugin**

In `bin/src/debug.rs`, extend the import block at the top and the `add_plugins` call inside `DebugPlugin::build`:

```rust
use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
```

```rust
app.add_plugins((
    FrameTimeDiagnosticsPlugin::default(),
    EntityCountDiagnosticsPlugin::default(),
    SystemInformationDiagnosticsPlugin,
))
.add_systems(Update, log_diagnostics);
```

- [x] **Step 3: Read CPU + memory in `log_diagnostics`**

Inside `log_diagnostics`, after the existing `entity_count` block, add:

```rust
let cpu_pct = diagnostics
    .get(&SystemInformationDiagnosticsPlugin::CPU_USAGE)
    .and_then(|d| d.smoothed())
    .unwrap_or(0.0);
let mem_mb = diagnostics
    .get(&SystemInformationDiagnosticsPlugin::MEM_USAGE)
    .and_then(|d| d.smoothed())
    .unwrap_or(0.0);
```

Then update the `info!` line to include them:

```rust
info!(
    "diag fps={fps:.1} frame_ms={frame_time_ms:.2} entities={entity_count:.0} \
     cpu={cpu_pct:.1}% mem={mem_mb:.0}MB elapsed={now:.0}s"
);
```

- [x] **Step 4: Build and run to verify the new fields populate**

Run: `cargo run -p kingdom --bin TheFifthKingdom 2>&1 | head -30`
Expected: after a few seconds, log lines like `diag fps=60.0 frame_ms=16.6 entities=5343 cpu=12.4% mem=820MB elapsed=2s`. The `cpu` and `mem` fields may read 0.0 on the very first sample (sysinfo polls at low frequency) but should populate within 5–10 seconds.

If the values stay at 0.0 indefinitely, the `sysinfo_plugin` feature isn't actually enabled — go back to Step 1.

- [x] **Step 5: Run the lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 6: Commit**

```
git add bin/src/debug.rs Cargo.toml
git commit -m "diag: log process CPU and memory alongside FPS and entity count"
```

(Drop `Cargo.toml` from the `git add` if Step 1 didn't require an edit.)

---

### Task 2: Tracy profiling feature

**Files:**
- Modify: `bin/Cargo.toml`
- Modify: `CLAUDE.md` (one-line usage doc)

**Design choice — feature flag vs always-on:**

- **Feature-gated `trace` (selected):** `bevy/trace_tracy` pulls in tracing-tracy and tracy-client. They have non-trivial overhead (a per-frame TCP connection to the Tracy GUI) and link against system networking. Gating behind `--features trace` keeps default builds clean and CI builds unaffected. Auto-selected — no downsides compared to alternatives. Let me know if you disagree.
- Always-on Tracy: forces every build to link tracy-client even when not profiling. Worse compile times, dead code in production.

Tracy itself runs as a separate GUI app. The Bevy book recommends `tracy-profiler` 0.11.x for Bevy 0.18; the version is pinned by the `bevy/trace_tracy` feature internally — we don't pin it ourselves.

- [x] **Step 1: Add the `trace` feature to `bin/Cargo.toml`**

In `bin/Cargo.toml`, extend the `[features]` block:

```toml
[features]
dev = ["bevy/dynamic_linking", "bevy/hotpatching", "bevy/dev"]
gen-atlas = ["dep:image"]
trace = ["bevy/trace_tracy"]
```

- [x] **Step 2: Verify the feature compiles**

Run: `cargo check -p kingdom --features trace`
Expected: compiles without errors. The first build will pull in `tracing-tracy` and `tracy-client` — that's normal.

- [x] **Step 3: Document usage in CLAUDE.md**

In `CLAUDE.md`, find the `## Build & Development Commands` section and append one row to the code block:

```bash
just dev              # Run with dynamic linking + hotpatching (fast iteration)
just run              # Run normally
just build            # Debug build
just build-release    # Release build
just test             # Run tests (cargo nextest)
just lint             # Format check + clippy
just fmt              # Auto-format (cargo fmt + taplo)
just watch            # Continuous build via bacon
cargo run --features trace  # Connect to Tracy GUI for per-system frame profiling
```

Add a short paragraph below the code block:

```markdown
For Tracy profiling, install the `tracy-profiler` GUI separately (Homebrew on
macOS: `brew install --cask tracy`; or download from the Tracy GitHub
releases). Launch the GUI first, then run the game with `--features trace` —
the game connects automatically.
```

- [x] **Step 4: Smoke-test the trace build**

Run: `cargo build -p kingdom --features trace 2>&1 | tail -5`
Expected: build succeeds. We don't actually launch Tracy in this step — that requires the GUI app, which is out of band. The build-success check is enough to verify the feature is wired.

- [x] **Step 5: Run the lint pass**

Run: `just lint`
Expected: no warnings. (`just lint` doesn't pass `--features trace`, so the trace-only code paths aren't linted here. That's fine — they're entirely inside Bevy.)

- [x] **Step 6: Commit**

```
git add bin/Cargo.toml CLAUDE.md
git commit -m "diag: add trace feature for Tracy frame profiling"
```

---

### Task 3: bevy_mod_debugdump CLI flag

**Files:**
- Modify: `bin/Cargo.toml`
- Modify: `bin/src/cli.rs`
- Modify: `bin/src/main.rs`

**Design choice — CLI flag vs key binding vs separate binary:**

- **CLI flag (selected):** `--dump-schedule <path>` builds the app, dumps the requested schedule to DOT, and exits before `app.run()`. No runtime cost in normal play. Composable with `cargo run --` so an engineer can do `cargo run -- --dump-schedule target/schedule.dot && dot -Tsvg target/schedule.dot -o target/schedule.svg`.
  - Pros: zero runtime overhead, scriptable, fits existing clap setup.
  - Cons: requires building the app to dump (no live updates while game runs — but the schedule is static, so live updates are not useful).
- Runtime key binding (rejected): the schedule graph is static — there's no point dumping it from a running game.
- Separate binary (rejected): duplicates plugin registration, drifts out of sync.

`bevy_mod_debugdump` for Bevy 0.18 is published at version `0.18.x`. Verify the exact patch version on crates.io at install time.

- [x] **Step 1: Add the dependency to `bin/Cargo.toml`**

In `bin/Cargo.toml`, extend the `[dependencies]` block. The crate has to match Bevy 0.18 — pick the latest `0.18.*` release on crates.io:

```toml
bevy_mod_debugdump = "0.18"
```

If `cargo search bevy_mod_debugdump` shows no `0.18.x` release, fall back to the highest published version that depends on `bevy = "0.18"` (check the crate's `Cargo.toml` on crates.io). Document the chosen version in the commit message.

- [x] **Step 2: Add the CLI flag**

Replace the body of `bin/src/cli.rs` with:

```rust
#[derive(clap::Parser)]
pub struct Args {
    #[arg(long)]
    pub seed: Option<u64>,

    /// Dump the Update schedule graph to the given path as DOT and exit.
    /// Convert with `dot -Tsvg <path> -o <path>.svg` to view.
    #[arg(long, value_name = "PATH")]
    pub dump_schedule: Option<std::path::PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parses_seed_flag() {
        let args = Args::try_parse_from(["kingdom", "--seed", "999"]).unwrap();
        assert_eq!(args.seed, Some(999));
    }

    #[test]
    fn cli_no_seed_flag_yields_none() {
        let args = Args::try_parse_from(["kingdom"]).unwrap();
        assert_eq!(args.seed, None);
    }

    #[test]
    fn cli_parses_dump_schedule_flag() {
        let args =
            Args::try_parse_from(["kingdom", "--dump-schedule", "schedule.dot"]).unwrap();
        assert_eq!(
            args.dump_schedule,
            Some(std::path::PathBuf::from("schedule.dot"))
        );
    }
}
```

- [x] **Step 3: Wire the dump path through `main`**

Replace `bin/src/main.rs` with:

```rust
use bevy::prelude::*;
use clap::Parser;
use kingdom_core::{LaunchConfig, default_seed};

mod cli;
mod debug;
mod plugins;

use cli::Args;
use debug::DebugPlugin;
use plugins::KingdomPlugins;

fn main() {
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(default_seed);

    let mut app = App::new();
    app.insert_resource(LaunchConfig { seed }).add_plugins((
        DefaultPlugins,
        KingdomPlugins,
        DebugPlugin,
    ));

    if let Some(path) = args.dump_schedule {
        let dot = bevy_mod_debugdump::schedule_graph_dot(
            &mut app,
            Update,
            &bevy_mod_debugdump::schedule_graph::Settings::default(),
        );
        std::fs::write(&path, dot).expect("write schedule DOT");
        eprintln!("schedule dumped to {}", path.display());
        return;
    }

    app.run();
}
```

Note: `bevy_mod_debugdump::schedule_graph_dot` builds plugins through `app.finish()` internally on most versions. If the API in the version you pinned differs (e.g. requires explicit `app.finish()` and `app.cleanup()` calls first, or returns `Result<String, _>` instead of `String`), adjust to match what `cargo doc -p bevy_mod_debugdump --open` shows. Don't guess — read the docs.

- [x] **Step 4: Run the CLI test**

Run: `cargo nextest run -p kingdom cli_parses_dump_schedule_flag`
Expected: PASS.

- [x] **Step 5: Smoke-test the dump end-to-end**

Run:
```
cargo run -p kingdom --bin TheFifthKingdom -- --dump-schedule target/schedule.dot
```
Expected: process exits within a few seconds (no game window) and `target/schedule.dot` exists with non-empty DOT content (`head -5 target/schedule.dot` should show `digraph` at the top).

- [x] **Step 6: Run the lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 7: Commit**

```
git add bin/Cargo.toml bin/src/cli.rs bin/src/main.rs
git commit -m "diag: add --dump-schedule flag using bevy_mod_debugdump"
```

If you also touched `Cargo.lock`, include it.

---

### Task 4: bevy-inspector-egui Ctrl+D toggle

**Files:**
- Modify: `bin/Cargo.toml`
- Modify: `bin/src/debug.rs`

**Design choice — quick `WorldInspectorPlugin` vs custom inspector vs nothing:**

- **`WorldInspectorPlugin` toggled by Ctrl+D (selected):** the `bevy_inspector_egui::quick::WorldInspectorPlugin` is a one-line drop-in. It registers a floating egui window with a tree of every entity, component, and resource. Toggling its visibility on a key avoids screen real estate cost during normal play and avoids egui drawing cost when hidden.
  - Pros: minimal code, mature crate, the same widget every Bevy dev recognises.
  - Cons: egui still ticks even when its windows are closed (small cost, ~0.1ms/frame on this workload).
- Custom inspector (rejected): re-implements what `bevy-inspector-egui` already does well.
- Nothing (rejected): runtime entity inspection is the second-most-useful tool after Tracy for catching the kind of bug the bacteria explosion was.

`bevy-inspector-egui` and `bevy_egui` versions must match Bevy 0.18. Verify on crates.io.

Bevy's built-in `input_toggle_active` only accepts a single key, so the chord (Ctrl+D) needs a small custom run-condition that latches a `Local<bool>` on `Ctrl+D` just-pressed. The plugin only adds its UI systems when the latch is active.

- [x] **Step 1: Add dependencies to `bin/Cargo.toml`**

In `bin/Cargo.toml`, extend `[dependencies]`. Pick the latest published version for each that depends on `bevy = "0.18"`:

```toml
bevy_egui = "0.39"
bevy-inspector-egui = "0.36"
```

If `cargo search` shows newer or older numbers as the Bevy 0.18-compatible pair, use those instead. The two crates' versions are coupled — `bevy-inspector-egui` declares which `bevy_egui` it needs; check its Cargo.toml on crates.io.

- [x] **Step 2: Wire the inspector into `DebugPlugin` with a Ctrl+D toggle**

In `bin/src/debug.rs`, extend the imports:

```rust
use bevy::input::ButtonInput;
use bevy::prelude::{KeyCode, Local, Res};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
```

Add a run-condition that latches on Ctrl+D just-pressed (place it near the bottom of `debug.rs`, alongside the other free functions):

```rust
fn inspector_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut active: Local<bool>,
) -> bool {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyD) {
        *active = !*active;
    }
    *active
}
```

Inside `DebugPlugin::build`, after the existing `add_plugins((...))` call but before `add_systems`, add:

```rust
app.add_plugins(WorldInspectorPlugin::new().run_if(inspector_toggle));
```

The latch starts `false` so the inspector is hidden; Ctrl+D flips it on, Ctrl+D again flips it off.

- [x] **Step 3: Build to confirm the deps resolve**

Run: `cargo check -p kingdom`
Expected: clean build. If it errors with a `bevy_egui` or `bevy_winit` version conflict, the published version of `bevy-inspector-egui` you pinned doesn't match the workspace's Bevy version — pick a different one.

- [x] **Step 4: Smoke-test the toggle in dev**

Run: `just dev`
Expected: game launches normally, no inspector visible. Press Ctrl+D once → inspector window appears with the entity tree. Press Ctrl+D again → window disappears. If you have no graphical environment (CI, headless), skip this step and note in your report.

- [x] **Step 5: Run the lint pass**

Run: `just lint`
Expected: no warnings.

- [x] **Step 6: Commit**

```
git add bin/Cargo.toml bin/src/debug.rs
git commit -m "diag: add bevy-inspector-egui world inspector behind Ctrl+D"
```

If you also touched `Cargo.lock`, include it.

---

## Final verification

After all four tasks land:

- [x] **Step 1: Run the full workspace test suite**

Run: `just test`
Expected: PASS, including the new `cli_parses_dump_schedule_flag` test.

- [x] **Step 2: Run the full lint pass**

Run: `just lint`
Expected: no warnings.

- [ ] **Step 3: Smoke-test all four tools in one dev session**

Run: `just dev`
Expected:
- Log lines now include `cpu=…% mem=…MB`.
- Press Ctrl+D → world inspector appears.
- Press Ctrl+D again → world inspector disappears.

Then in a separate terminal:

```
cargo run -p kingdom --bin TheFifthKingdom -- --dump-schedule target/schedule.dot
```
Expected: exits quickly, `target/schedule.dot` is non-empty.

Skip Step 3 if you have no graphical environment.

- [x] **Step 4: Smoke-test the trace build compiles**

Run: `cargo build -p kingdom --features trace`
Expected: build succeeds. Connecting to Tracy GUI is a manual step the engineer does when they actually want to profile — the build-success check is enough here.
