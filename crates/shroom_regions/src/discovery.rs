use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use shroom_core::{
    DecompositionComplete, GridPos, GridWorld, HyphalTip, Occupant, RegionStates,
    SlotMachineTriggered, SpecializationType, StudyComplete, Tile, TileContents, TileDiscovered,
    UnlockPool,
};

const STUDY_RATE: f32 = 0.1;
const DECOMP_RATE: f32 = 0.1;

pub fn explorer_discovery_system(
    tips: Query<(&GridPos, &HyphalTip)>,
    mut tiles: Query<&mut Tile>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
    mut discovered_messages: MessageWriter<TileDiscovered>,
) {
    for (gpos, tip) in tips.iter() {
        let is_explorer = region_states
            .get(tip.region_id)
            .is_some_and(|r| r.specialization == Some(SpecializationType::Explorer));
        if !is_explorer {
            continue;
        }

        let positions: Vec<IVec2> = std::iter::once(gpos.0)
            .chain(grid.neighbors(gpos.0).map(|(p, _)| p))
            .collect();

        for pos in positions {
            if let Some(&entity) = grid.tiles.get(&pos) {
                if let Ok(mut tile) = tiles.get_mut(entity) {
                    if !tile.discovered {
                        tile.discovered = true;
                        discovered_messages.write(TileDiscovered {
                            pos,
                            contents: tile.contents,
                        });
                    }
                }
            }
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct StudyProgress {
    pub entries: HashMap<IVec2, f32>,
}

pub fn researcher_study_system(
    tiles: Query<(&GridPos, &Tile)>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
    mut study: ResMut<StudyProgress>,
    mut study_messages: MessageWriter<StudyComplete>,
) {
    for (gpos, tile) in tiles.iter() {
        let Occupant::Player(rid) = tile.occupant else {
            continue;
        };
        let is_researcher = region_states
            .get(rid)
            .is_some_and(|r| r.specialization == Some(SpecializationType::Researcher));
        if !is_researcher {
            continue;
        }

        for (npos, nentity) in grid.neighbors(gpos.0) {
            if let Ok((_, ntile)) = tiles.get(nentity) {
                if !ntile.discovered || ntile.contents.is_none() {
                    continue;
                }
                let progress = study.entries.entry(npos).or_insert(0.0);
                *progress += STUDY_RATE;
                if *progress >= 1.0 {
                    let pool = match ntile.contents {
                        Some(TileContents::OrganicMatter) => UnlockPool::Organic,
                        Some(TileContents::Mineral) => UnlockPool::Mineral,
                        Some(TileContents::Artifact | TileContents::Fragment(_)) => {
                            UnlockPool::Ruins
                        }
                        _ => UnlockPool::Organic,
                    };
                    study_messages.write(StudyComplete { pos: npos, pool });
                    study.entries.remove(&npos);
                }
            }
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct DecompProgress {
    pub entries: HashMap<IVec2, f32>,
}

pub fn decomposer_discovery_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    region_states: Res<RegionStates>,
    mut progress: ResMut<DecompProgress>,
    mut decomp_messages: MessageWriter<DecompositionComplete>,
    mut slot_messages: MessageWriter<SlotMachineTriggered>,
) {
    for (gpos, mut tile) in tiles.iter_mut() {
        let Occupant::Player(rid) = tile.occupant else {
            continue;
        };
        let is_decomposer = region_states
            .get(rid)
            .is_some_and(|r| r.specialization == Some(SpecializationType::Decomposer));
        if !is_decomposer {
            continue;
        }

        if !matches!(tile.contents, Some(TileContents::UniqueDecomposable(_))) {
            continue;
        }

        let prog = progress.entries.entry(gpos.0).or_insert(0.0);
        *prog += DECOMP_RATE;
        if *prog >= 1.0 {
            tile.contents = None;
            progress.entries.remove(&gpos.0);
            decomp_messages.write(DecompositionComplete { pos: gpos.0 });
            slot_messages.write(SlotMachineTriggered {
                pool: UnlockPool::Decomposition,
                options: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<StudyProgress>();
        app.init_resource::<DecompProgress>();
        app.add_message::<TileDiscovered>();
        app.add_message::<StudyComplete>();
        app.add_message::<DecompositionComplete>();
        app.add_message::<SlotMachineTriggered>();
        app
    }

    #[test]
    fn explorer_tip_discovers_tile() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().specialization = Some(SpecializationType::Explorer);

        let pos = IVec2::new(3, 3);
        let entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    discovered: false,
                    contents: Some(TileContents::Mineral),
                    occupant: Occupant::Player(rid),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);

        app.world_mut().spawn((
            GridPos(pos),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));

        app.add_systems(Update, explorer_discovery_system);
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(tile.discovered, "tile should be marked discovered");
    }

    #[test]
    fn researcher_completes_study() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().specialization = Some(SpecializationType::Researcher);

        let pos = IVec2::new(5, 5);
        let neighbor_pos = IVec2::new(6, 5);

        let player_entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, player_entity);

        let neighbor_entity = app
            .world_mut()
            .spawn((
                GridPos(neighbor_pos),
                Tile {
                    discovered: true,
                    contents: Some(TileContents::Mineral),
                    occupant: Occupant::Player(rid),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(neighbor_pos, neighbor_entity);

        app.world_mut()
            .resource_mut::<StudyProgress>()
            .entries
            .insert(neighbor_pos, 0.95);

        app.add_systems(Update, researcher_study_system);
        app.update();

        let study = app.world().resource::<StudyProgress>();
        let done = study.entries.get(&neighbor_pos).is_none_or(|&v| v >= 1.0);
        assert!(done, "study should complete when progress reaches 1.0");
    }

    #[test]
    fn decomposer_breaks_down_unique_decomposable() {
        let mut app = test_app();
        let mut rs = app.world_mut().resource_mut::<RegionStates>();
        let rid = rs.create_region();
        rs.get_mut(rid).unwrap().specialization = Some(SpecializationType::Decomposer);

        let pos = IVec2::new(2, 2);
        let entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    contents: Some(TileContents::UniqueDecomposable(0)),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);

        app.world_mut()
            .resource_mut::<DecompProgress>()
            .entries
            .insert(pos, 0.95);

        app.add_systems(Update, decomposer_discovery_system);
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(
            tile.contents.is_none()
                || !matches!(tile.contents, Some(TileContents::UniqueDecomposable(_))),
            "decomposable should be consumed on completion"
        );
    }
}
