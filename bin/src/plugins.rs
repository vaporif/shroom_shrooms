use bevy::app::{PluginGroup, PluginGroupBuilder};

use kingdom_ai::{EnvironmentPlugin, OrganismsPlugin};
use kingdom_core::{CorePlugin, SimulationPlugin};
use kingdom_fruiting::FruitingPlugin;
use kingdom_growth::GrowthPlugin;
use kingdom_input::InputPlugin;
use kingdom_regions::RegionsPlugin;
use kingdom_render::RenderPlugin;
use kingdom_ui::UiPlugin;
use kingdom_world::WorldPlugin;

pub struct KingdomPlugins;

impl PluginGroup for KingdomPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(CorePlugin)
            .add(SimulationPlugin)
            .add(WorldPlugin)
            .add(GrowthPlugin)
            .add(RegionsPlugin)
            .add(RenderPlugin)
            .add(InputPlugin)
            .add(OrganismsPlugin)
            .add(EnvironmentPlugin)
            .add(FruitingPlugin)
            .add(UiPlugin)
    }
}
