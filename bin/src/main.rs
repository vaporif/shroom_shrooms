use bevy::prelude::*;
use clap::Parser;
use kingdom_core::{default_seed, LaunchConfig};

mod cli;
mod debug;
mod plugins;

use cli::Args;
use debug::DebugPlugin;
use plugins::KingdomPlugins;

fn main() {
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(default_seed);
    App::new()
        .insert_resource(LaunchConfig { seed })
        .add_plugins((DefaultPlugins, KingdomPlugins, DebugPlugin))
        .run();
}
