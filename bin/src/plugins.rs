use bevy::app::{PluginGroup, PluginGroupBuilder};

use fungai_ai::AiPlugin;
use fungai_core::{CorePlugin, SimulationPlugin};
use fungai_fruiting::FruitingPlugin;
use fungai_growth::GrowthPlugin;
use fungai_input::InputPlugin;
use fungai_regions::RegionsPlugin;
use fungai_render::RenderPlugin;
use fungai_ui::UiPlugin;
use fungai_world::WorldPlugin;

pub struct FungaiPlugins;

impl PluginGroup for FungaiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(CorePlugin)
            .add(SimulationPlugin)
            .add(WorldPlugin)
            .add(GrowthPlugin)
            .add(RegionsPlugin)
            .add(RenderPlugin)
            .add(InputPlugin)
            .add(AiPlugin)
            .add(FruitingPlugin)
            .add(UiPlugin)
    }
}
