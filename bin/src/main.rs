use bevy::prelude::*;

mod plugins;

use plugins::FungaiPlugins;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FungaiPlugins))
        .run();
}
