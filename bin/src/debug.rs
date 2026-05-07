use bevy::diagnostic::{
    DiagnosticPath, DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            SystemInformationDiagnosticsPlugin,
        ));
        app.add_plugins(WorldInspectorPlugin::new().run_if(inspector_toggle));
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

    let components = world.components();
    let mut buckets: Vec<(String, u32)> = world
        .archetypes()
        .iter()
        .filter(|a| !a.is_empty())
        .map(|archetype| {
            let mut names: Vec<String> = archetype
                .components()
                .iter()
                .filter_map(|cid| {
                    components
                        .get_info(*cid)
                        .map(|info| info.name().shortname().to_string())
                })
                .collect();
            names.sort_unstable();
            names.dedup();
            let label = if names.is_empty() {
                "<empty>".to_string()
            } else if names.len() <= 4 {
                names.join("+")
            } else {
                format!(
                    "{}+{}+{}+...({} more)",
                    names[0],
                    names[1],
                    names[2],
                    names.len() - 3
                )
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

fn inspector_toggle(keys: Res<ButtonInput<KeyCode>>, mut active: Local<bool>) -> bool {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyD) {
        *active = !*active;
    }
    *active
}
