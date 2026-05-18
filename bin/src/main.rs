use bevy::prelude::*;
use clap::Parser;
use kingdom_core::{
    DEFAULT_MAP_HEIGHT, DEFAULT_MAP_WIDTH, LaunchConfig, default_hive_count, default_seed,
};

mod cli;
mod debug;
mod plugins;

use cli::Args;
use debug::DebugPlugin;
use plugins::KingdomPlugins;

fn main() {
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(default_seed);
    let width = args.width.unwrap_or(DEFAULT_MAP_WIDTH);
    let height = args.height.unwrap_or(DEFAULT_MAP_HEIGHT);
    let hives = args
        .hives
        .unwrap_or_else(|| default_hive_count(width, height))
        .max(1);

    let mut app = App::new();
    app.insert_resource(LaunchConfig {
        seed,
        width,
        height,
        hives,
    })
    .add_plugins((DefaultPlugins, KingdomPlugins, DebugPlugin));

    if let Some(path) = args.dump_schedule {
        let dot = bevy_mod_debugdump::schedule_graph_dot(
            &mut app,
            Update,
            &bevy_mod_debugdump::schedule_graph::Settings::default(),
        );
        std::fs::write(&path, dot)
            .unwrap_or_else(|e| panic!("write schedule DOT to {}: {e}", path.display()));
        eprintln!("schedule dumped to {}", path.display());
        return;
    }

    app.run();
}
