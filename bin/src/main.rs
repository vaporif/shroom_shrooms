use bevy::prelude::*;
use clap::Parser;
use fungai_core::{LaunchConfig, default_seed};

mod cli;
mod plugins;

use cli::Args;
use plugins::FungaiPlugins;

fn main() {
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(default_seed);
    App::new()
        .insert_resource(LaunchConfig { seed })
        .add_plugins((DefaultPlugins, FungaiPlugins))
        .run();
}
