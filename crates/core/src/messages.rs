use bevy::ecs::message::Message;

use crate::grid::Hex;
use crate::tile::{FragmentId, TileContents};

#[derive(Message)]
pub struct TurnAdvanced;

#[derive(Message)]
pub struct TileDiscovered {
    pub pos: Hex,
    pub contents: Option<TileContents>,
}

#[derive(Message)]
pub struct DecompositionComplete {
    pub pos: Hex,
    pub was_unique: bool,
}

#[derive(Message)]
pub struct FragmentFused {
    pub fragment_id: FragmentId,
}
