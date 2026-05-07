use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum UnlockPool {
    Organic,
    Mineral,
    Ruins,
    Decomposition,
}

#[derive(Clone, Debug, Reflect)]
pub struct UnlockOption {
    pub name: String,
    pub description: String,
    pub pool: UnlockPool,
}
