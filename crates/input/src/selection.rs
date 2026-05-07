use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use kingdom_core::*;

use crate::wisp::TileTapped;

pub fn selection_system(
    mut taps: MessageReader<TileTapped>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut selected: ResMut<SelectedRegion>,
) {
    let Some(tap) = taps.read().last() else {
        return;
    };
    let Some(&entity) = grid.tiles.get(&tap.pos) else {
        return;
    };
    if let Ok(tile) = tiles.get(entity) {
        selected.selected_pos = Some(tap.pos);
        selected.region_id = tile.region_id;
    }
}
