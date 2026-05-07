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
        std::fs::write(&path, dot)
            .unwrap_or_else(|e| panic!("write schedule DOT to {}: {e}", path.display()));
        eprintln!("schedule dumped to {}", path.display());
        return;
    }

    app.run();
}
