use bevy::prelude::*;
use kingdom_core::{
    CLAIM_THRESHOLD, GridPos, GridWorld, Hex, MushroomEntity, RegionId, RegionStates,
    SPORE_RELAY_ACCURACY_RADIUS, Tile,
};
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::seq::IteratorRandom;

#[derive(Resource, Debug, Clone, Reflect)]
pub struct SporeAction {
    pub cooldown_remaining: u32,
    pub cooldown_max: u32,
    pub triggered: bool,
}

impl Default for SporeAction {
    fn default() -> Self {
        Self {
            cooldown_remaining: 0,
            cooldown_max: 10,
            triggered: false,
        }
    }
}

#[derive(Resource)]
pub struct SporeRng(pub StdRng);

impl Default for SporeRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(0))
    }
}

pub fn spore_system(
    mushrooms: Query<&MushroomEntity>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
    mut rng: ResMut<SporeRng>,
    mut spore_action: ResMut<SporeAction>,
) {
    // Tick down cooldown
    if spore_action.cooldown_remaining > 0 {
        spore_action.cooldown_remaining -= 1;
    }

    // Only fire when player triggers and cooldown is ready
    if !spore_action.triggered || spore_action.cooldown_remaining > 0 {
        spore_action.triggered = false;
        return;
    }

    // Pick one random mushroom to fire from
    let Some(mushroom) = mushrooms.iter().choose(&mut rng.0) else {
        spore_action.triggered = false;
        return;
    };

    let owning_region = find_owning_region(&tiles, mushroom.pos);
    let Some(region_id) = owning_region else {
        spore_action.triggered = false;
        return;
    };

    if region_states.get(region_id).is_none() {
        spore_action.triggered = false;
        return;
    }

    let landing_pos = pick_spore_landing(&tiles, mushroom.pos, &mut rng.0);

    if let Some(landing_pos) = landing_pos
        && let Some(&entity) = grid.tiles.get(&landing_pos)
        && let Ok((_, mut tile)) = tiles.get_mut(entity)
    {
        // Seed the landing tile so density flow can pick up from here:
        // claim it for the region by setting biomass above CLAIM_THRESHOLD.
        tile.region_id = Some(region_id);
        tile.biomass = tile.biomass.max(CLAIM_THRESHOLD + 0.2);
    }

    spore_action.triggered = false;
    spore_action.cooldown_remaining = spore_action.cooldown_max;
}

// THRESHOLD-GATED: only count a region as owning the area if its network has
// actually arrived (biomass past CLAIM_THRESHOLD), not just a sub-threshold tag.
fn find_owning_region(tiles: &Query<(&GridPos, &mut Tile)>, pos: Hex) -> Option<RegionId> {
    for (gpos, tile) in tiles.iter() {
        let dist = gpos.0.unsigned_distance_to(pos);
        if dist <= 3
            && let Some(rid) = tile.region_id
            && tile.biomass >= CLAIM_THRESHOLD
        {
            return Some(rid);
        }
    }
    None
}

fn pick_spore_landing(
    tiles: &Query<(&GridPos, &mut Tile)>,
    origin: Hex,
    rng: &mut StdRng,
) -> Option<Hex> {
    let radius = SPORE_RELAY_ACCURACY_RADIUS as u32;
    tiles
        .iter()
        .filter_map(|(gpos, tile)| {
            let dist = gpos.0.unsigned_distance_to(origin);
            (dist <= radius && tile.terrain.is_passable() && tile.region_id.is_none())
                .then_some(gpos.0)
        })
        .choose(rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{FragmentId, TerrainType};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<SporeAction>();
        app.insert_resource(SporeRng(StdRng::seed_from_u64(42)));
        app
    }

    fn spawn_tile_at(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let entity = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);
        entity
    }

    /// Spawn a ring of hex tiles around `center` up to `radius` hex distance.
    fn spawn_hex_area(app: &mut App, center: Hex, radius: u32, tile_fn: impl Fn(Hex) -> Tile) {
        for q in -(radius as i32)..=(radius as i32) {
            for r in -(radius as i32)..=(radius as i32) {
                let h = Hex::new(center.x + q, center.y + r);
                if h.unsigned_distance_to(center) <= radius {
                    spawn_tile_at(app, h, tile_fn(h));
                }
            }
        }
    }

    fn count_newly_claimed(app: &mut App, center: Hex, rid: RegionId) -> u32 {
        let mut count = 0;
        for (gpos, tile) in app
            .world_mut()
            .query::<(&GridPos, &Tile)>()
            .iter(app.world())
        {
            if gpos.0 != center
                && tile.region_id == Some(rid)
                && tile.biomass >= CLAIM_THRESHOLD + 0.1
            {
                count += 1;
            }
        }
        count
    }

    #[test]
    fn spore_seeds_tile_near_mushroom() {
        let mut app = test_app();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let center = Hex::new(5, 5);
        spawn_tile_at(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                ..default()
            },
        );

        // Fill area around center with empty passable tiles
        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                // Already spawned above, but spawn_tile_at will just overwrite the grid entry
                Tile {
                    region_id: Some(rid),
                    biomass: 0.5,
                    ..default()
                }
            } else {
                Tile::default()
            }
        });

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: center,
            vision_radius: 10.0,
        });

        app.world_mut().resource_mut::<SporeAction>().triggered = true;

        app.add_systems(Update, spore_system);
        app.update();

        let claimed = count_newly_claimed(&mut app, center, rid);
        assert_eq!(claimed, 1, "spore should claim exactly one tile");

        let (claimed_pos, claimed_tile) = app
            .world_mut()
            .query::<(&GridPos, &Tile)>()
            .iter(app.world())
            .find(|(gpos, t)| {
                gpos.0 != center && t.region_id == Some(rid) && t.biomass >= CLAIM_THRESHOLD + 0.1
            })
            .map(|(gp, t)| (gp.0, t.clone()))
            .expect("should have exactly one newly-claimed tile");
        assert_eq!(
            claimed_tile.region_id,
            Some(rid),
            "claim should belong to mushroom's region"
        );

        let dist = claimed_pos.unsigned_distance_to(center);
        assert!(
            dist <= SPORE_RELAY_ACCURACY_RADIUS as u32,
            "claim should land within spore accuracy radius"
        );
    }

    #[test]
    fn spore_does_not_land_on_impassable_tile() {
        let mut app = test_app();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let center = Hex::new(5, 5);
        spawn_tile_at(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                ..default()
            },
        );

        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                Tile {
                    region_id: Some(rid),
                    biomass: 0.5,
                    ..default()
                }
            } else {
                Tile {
                    terrain: TerrainType::Rock,
                    ..default()
                }
            }
        });

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: center,
            vision_radius: 10.0,
        });

        app.world_mut().resource_mut::<SporeAction>().triggered = true;

        app.add_systems(Update, spore_system);
        app.update();

        let claimed = count_newly_claimed(&mut app, center, rid);
        assert_eq!(
            claimed, 0,
            "no tile should be claimed when all nearby tiles are impassable"
        );
    }

    #[test]
    fn spore_skips_mushroom_without_owning_region() {
        let mut app = test_app();

        let center = Hex::new(5, 5);
        spawn_hex_area(&mut app, center, 3, |_| Tile::default());

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: center,
            vision_radius: 10.0,
        });

        app.world_mut().resource_mut::<SporeAction>().triggered = true;

        app.add_systems(Update, spore_system);
        app.update();

        let mut any_claimed = false;
        for (_gpos, tile) in app
            .world_mut()
            .query::<(&GridPos, &Tile)>()
            .iter(app.world())
        {
            if tile.region_id.is_some() && tile.biomass >= CLAIM_THRESHOLD {
                any_claimed = true;
            }
        }
        assert!(
            !any_claimed,
            "no tile should be claimed when mushroom has no owning region"
        );
    }

    #[test]
    fn no_crash_with_no_mushrooms() {
        let mut app = test_app();
        spawn_tile_at(&mut app, Hex::ZERO, Tile::default());
        app.world_mut().resource_mut::<SporeAction>().triggered = true;
        app.add_systems(Update, spore_system);
        app.update();
    }

    #[test]
    fn spore_does_not_fire_without_trigger() {
        let mut app = test_app();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let center = Hex::new(5, 5);
        spawn_tile_at(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                ..default()
            },
        );

        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                Tile {
                    region_id: Some(rid),
                    biomass: 0.5,
                    ..default()
                }
            } else {
                Tile::default()
            }
        });

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: center,
            vision_radius: 10.0,
        });

        // Not triggered -- should not fire
        app.add_systems(Update, spore_system);
        app.update();

        let claimed = count_newly_claimed(&mut app, center, rid);
        assert_eq!(claimed, 0, "spore should not fire without trigger");
    }

    #[test]
    fn spore_cooldown_prevents_rapid_fire() {
        let mut app = test_app();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let center = Hex::new(5, 5);
        spawn_tile_at(
            &mut app,
            center,
            Tile {
                region_id: Some(rid),
                biomass: 0.5,
                ..default()
            },
        );

        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                Tile {
                    region_id: Some(rid),
                    biomass: 0.5,
                    ..default()
                }
            } else {
                Tile::default()
            }
        });

        app.world_mut().spawn(MushroomEntity {
            fragment_id: FragmentId(0),
            pos: center,
            vision_radius: 10.0,
        });

        app.add_systems(Update, spore_system);

        // First trigger -- should fire
        app.world_mut().resource_mut::<SporeAction>().triggered = true;
        app.update();

        let claimed = count_newly_claimed(&mut app, center, rid);
        assert_eq!(claimed, 1, "first trigger should claim a tile");

        // Immediately trigger again -- should be blocked by cooldown
        app.world_mut().resource_mut::<SporeAction>().triggered = true;
        app.update();

        let claimed = count_newly_claimed(&mut app, center, rid);
        assert_eq!(claimed, 1, "second trigger should be blocked by cooldown");

        let action = app.world().resource::<SporeAction>();
        assert!(
            action.cooldown_remaining > 0,
            "cooldown should still be active"
        );
    }
}
