use std::collections::BTreeSet;

use bevy::diagnostic::{
    DiagnosticPath, DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{EguiGlobalSettings, EguiPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use kingdom_input::Action;
use leafwing_input_manager::plugin::InputManagerSystem;
use leafwing_input_manager::prelude::ActionState;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            SystemInformationDiagnosticsPlugin,
        ));
        app.insert_resource(EguiGlobalSettings {
            enable_absorb_bevy_input_system: true,
            ..default()
        });
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(WorldInspectorPlugin::new().run_if(inspector_toggle));
        app.add_systems(
            PreUpdate,
            gate_game_input_to_egui.in_set(InputManagerSystem::ManualControl),
        );
        app.add_systems(Update, log_diagnostics);
    }
}

fn log_diagnostics(world: &World, mut last_log: Local<f32>) {
    let now = world.resource::<Time>().elapsed_secs();
    if now - *last_log < 2.0 {
        return;
    }
    *last_log = now;

    let diagnostics = world.resource::<DiagnosticsStore>();
    let fps = smoothed(diagnostics, &FrameTimeDiagnosticsPlugin::FPS);
    let frame_time_ms = smoothed(diagnostics, &FrameTimeDiagnosticsPlugin::FRAME_TIME);
    let entity_count = smoothed(diagnostics, &EntityCountDiagnosticsPlugin::ENTITY_COUNT);
    let cpu_pct = smoothed(
        diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
    );
    let mem_mib = smoothed(
        diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    ) * 1024.0;

    info!(
        "diag fps={fps:.1} frame_ms={frame_time_ms:.2} entities={entity_count:.0} \
         cpu={cpu_pct:.1}% mem={mem_mib:.0}MiB elapsed={now:.0}s"
    );

    log_archetype_buckets(world);
}

fn log_archetype_buckets(world: &World) {
    let components = world.components();
    let mut buckets: Vec<(String, u32)> = world
        .archetypes()
        .iter()
        .filter(|a| !a.is_empty())
        .map(|archetype| {
            let names: BTreeSet<String> = archetype
                .components()
                .iter()
                .filter_map(|cid| {
                    components
                        .get_info(*cid)
                        .map(|info| info.name().shortname().to_string())
                })
                .collect();
            let label = if names.is_empty() {
                "<empty>".to_string()
            } else if names.len() <= 4 {
                names.into_iter().collect::<Vec<_>>().join("+")
            } else {
                let head: Vec<String> = names.iter().take(3).cloned().collect();
                format!("{}+...({} more)", head.join("+"), names.len() - 3)
            };
            (label, archetype.len())
        })
        .collect();
    buckets.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));
    for (label, count) in buckets.iter().take(8) {
        info!("  archetype count={count} :: {label}");
    }
}

fn smoothed(diagnostics: &DiagnosticsStore, path: &DiagnosticPath) -> f64 {
    diagnostics
        .get(path)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0)
}

fn gate_game_input_to_egui(
    egui_wants: Res<EguiWantsInput>,
    mut actions: ResMut<ActionState<Action>>,
) {
    let egui_owns = egui_wants.wants_any_pointer_input() || egui_wants.wants_any_keyboard_input();
    if egui_owns {
        actions.disable();
    } else {
        actions.enable();
    }
}

fn inspector_toggle(keys: Res<ButtonInput<KeyCode>>, mut active: Local<bool>) -> bool {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyI) {
        *active = !*active;
    }
    *active
}
