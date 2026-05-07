use bevy::app::{App, Plugin, Update};
use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
};
use bevy::ecs::system::Local;
use bevy::ecs::world::World;
use bevy::log::info;
use bevy::time::Time;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
        ))
        .add_systems(Update, log_diagnostics);
    }
}

fn log_diagnostics(world: &mut World, mut last_log: Local<f32>) {
    let now = world.resource::<Time>().elapsed_secs();
    if now - *last_log < 2.0 {
        return;
    }
    *last_log = now;

    let diagnostics = world.resource::<DiagnosticsStore>();
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    info!(
        "diag fps={fps:.1} frame_ms={frame_time_ms:.2} entities={entity_count:.0} elapsed={now:.0}s"
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
    buckets.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    for (label, count) in buckets.iter().take(8) {
        info!("  archetype count={count} :: {label}");
    }
}
