use bevy::ecs::message::Message;

use crate::abilities::UnlockPool;
use crate::grid::Hex;
use crate::region::RegionId;
use crate::tile::{FragmentId, TileContents};

#[derive(Message)]
pub struct TurnAdvanced;

#[derive(Message)]
pub struct TileDiscovered {
    pub pos: Hex,
    pub contents: Option<TileContents>,
}

#[derive(Message)]
pub struct StudyComplete {
    pub pos: Hex,
    pub pool: UnlockPool,
}

#[derive(Message)]
pub struct DecompositionComplete {
    pub pos: Hex,
}

#[derive(Message)]
pub struct FragmentFused {
    pub fragment_id: FragmentId,
}

#[derive(Message)]
pub struct NeutralFungiMerged {
    pub fungus_id: u32,
    pub region_id: RegionId,
}
