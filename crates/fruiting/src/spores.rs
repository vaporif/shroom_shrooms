use bevy::prelude::*;
use fungai_core::{
    GridPos, GridWorld, Hex, HyphalTip, MushroomEntity, Occupant, RegionId, RegionStates,
    SPORE_RELAY_ACCURACY_RADIUS, SporeAction, Tile,
};
use rand::prelude::*;
use rand::rngs::StdRng;

#[derive(Resource)]
pub struct SporeRng(pub StdRng);

impl Default for SporeRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(0))
    }
}

pub fn spore_system(
    mushrooms: Query<&MushroomEntity>,
    tiles: Query<(&GridPos, &Tile)>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
    mut commands: Commands,
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
    let mufungai_list: Vec<&MushroomEntity> = mushrooms.iter().collect();
    if mufungai_list.is_empty() {
        spore_action.triggered = false;
        return;
    }

    let idx = rng.0.random_range(0..mufungai_list.len());
    let mushroom = mufungai_list[idx];

    let owning_region = find_owning_region(&tiles, mushroom.pos);
    let Some(region_id) = owning_region else {
        spore_action.triggered = false;
        return;
    };

    if region_states.get(region_id).is_none() {
        spore_action.triggered = false;
        return;
    }

    if let Some(landing_pos) = pick_spore_landing(&grid, &tiles, mushroom.pos, &mut rng.0) {
        commands.spawn((GridPos(landing_pos), HyphalTip { region_id, age: 0 }));
    }

    spore_action.triggered = false;
    spore_action.cooldown_remaining = spore_action.cooldown_max;
}

fn find_owning_region(tiles: &Query<(&GridPos, &Tile)>, pos: Hex) -> Option<RegionId> {
    for (gpos, tile) in tiles.iter() {
        let dist = gpos.0.unsigned_distance_to(pos);
        if dist <= 3
            && let Occupant::Player(rid) = tile.occupant
        {
            return Some(rid);
        }
    }
    None
}

fn pick_spore_landing(
    _grid: &GridWorld,
    tiles: &Query<(&GridPos, &Tile)>,
    origin: Hex,
    rng: &mut StdRng,
) -> Option<Hex> {
    let radius = SPORE_RELAY_ACCURACY_RADIUS as u32;

    let candidates: Vec<Hex> = tiles
        .iter()
        .filter_map(|(gpos, tile)| {
            let dist = gpos.0.unsigned_distance_to(origin);
            if dist <= radius
                && tile.terrain.is_passable()
                && !tile.occupant.is_player()
                && !tile.occupant.is_rival()
            {
                Some(gpos.0)
            } else {
                None
            }
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let idx = rng.random_range(0..candidates.len());
    Some(candidates[idx])
}

#[cfg(test)]
mod tests {
    use super::*;
    use fungai_core::{FragmentId, TerrainType};

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

    #[test]
    fn spore_spawns_tip_near_mushroom() {
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
                occupant: Occupant::Player(rid),
                ..default()
            },
        );

        // Fill area around center with empty passable tiles
        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                // Already spawned above, but spawn_tile_at will just overwrite the grid entry
                Tile {
                    occupant: Occupant::Player(rid),
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

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(tip_count, 1, "spore should spawn exactly one hyphal tip");

        let (tip_pos, tip) = app
            .world_mut()
            .query::<(&GridPos, &HyphalTip)>()
            .single(app.world())
            .expect("should have exactly one hyphal tip");
        assert_eq!(tip.region_id, rid, "tip should belong to mushroom's region");

        let dist = tip_pos.0.unsigned_distance_to(center);
        assert!(
            dist <= SPORE_RELAY_ACCURACY_RADIUS as u32,
            "tip should land within spore accuracy radius"
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
                occupant: Occupant::Player(rid),
                ..default()
            },
        );

        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                Tile {
                    occupant: Occupant::Player(rid),
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

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(
            tip_count, 0,
            "no tip should spawn when all nearby tiles are impassable"
        );
    }

    #[test]
    fn spore_skips_mufungai_without_owning_region() {
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

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(
            tip_count, 0,
            "no tip should spawn when mushroom has no owning region"
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
                occupant: Occupant::Player(rid),
                ..default()
            },
        );

        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                Tile {
                    occupant: Occupant::Player(rid),
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

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(tip_count, 0, "spore should not fire without trigger");
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
                occupant: Occupant::Player(rid),
                ..default()
            },
        );

        spawn_hex_area(&mut app, center, 3, |h| {
            if h == center {
                Tile {
                    occupant: Occupant::Player(rid),
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

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(tip_count, 1, "first trigger should spawn a tip");

        // Immediately trigger again -- should be blocked by cooldown
        app.world_mut().resource_mut::<SporeAction>().triggered = true;
        app.update();

        let tip_count = app
            .world_mut()
            .query::<&HyphalTip>()
            .iter(app.world())
            .count();
        assert_eq!(tip_count, 1, "second trigger should be blocked by cooldown");

        let action = app.world().resource::<SporeAction>();
        assert!(
            action.cooldown_remaining > 0,
            "cooldown should still be active"
        );
    }
}
