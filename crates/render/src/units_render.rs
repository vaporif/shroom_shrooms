use bevy::prelude::*;
use kingdom_core::{GridPos, HexLayout, Hive, SelectedUnit, Unit, UnitMovement};

use crate::assets::EntitySprites;
use crate::entity_render::organism_sprite_size;

/// Z layer for the unit-layer sprites, above terrain/network.
const HIVE_Z: f32 = 1.5;

/// Links a hive sprite to its source `Hive` entity.
///
/// Unlike `OrganismSpriteLink`, this has no `RemovedComponents`-based orphan
/// cleanup: hives are placed at world-gen and never despawned in Phase 1, so
/// the source entity always outlives the sprite.
#[derive(Component)]
pub struct HiveSprite(pub Entity);

pub fn spawn_hive_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_hives: Query<(Entity, &GridPos), Added<Hive>>,
) {
    let size = organism_sprite_size(&layout);
    for (source, gpos) in new_hives.iter() {
        let world = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            HiveSprite(source),
            Sprite {
                image: sprites.hive.clone(),
                color: neutral_tint(),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world.extend(HIVE_Z)),
        ));
    }
}

fn neutral_tint() -> Color {
    Color::srgb(0.55, 0.55, 0.55)
}

/// Recolour a hive sprite when its capture state changes.
pub fn hive_tint_system(
    hives: Query<&Hive, Changed<Hive>>,
    mut sprites: Query<(&HiveSprite, &mut Sprite)>,
) {
    if hives.is_empty() {
        return;
    }
    for (link, mut sprite) in &mut sprites {
        let Ok(hive) = hives.get(link.0) else {
            continue;
        };
        sprite.color = match hive.captured_by {
            Some(rid) => region_tint(rid.0),
            None => neutral_tint(),
        };
    }
}

/// Deterministic per-region hue so different networks read distinctly.
fn region_tint(id: u32) -> Color {
    let hue = (id as f32 * 67.0) % 360.0;
    Color::hsl(hue, 0.6, 0.55)
}

const UNIT_Z: f32 = 2.5;

/// Units render much smaller than a hex — a small body that visibly walks
/// across the hex it is crossing, rather than a sprite that fills the tile.
/// Fraction of the organism (hex-scale) sprite size; tuning value.
const UNIT_SPRITE_FRACTION: f32 = 0.2;

#[derive(Component)]
pub struct UnitSprite(pub Entity);

pub fn spawn_unit_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_units: Query<(Entity, &GridPos), Added<Unit>>,
) {
    let size = organism_sprite_size(&layout) * UNIT_SPRITE_FRACTION;
    for (source, gpos) in new_units.iter() {
        let world = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            UnitSprite(source),
            Sprite {
                image: sprites.fauna.clone(),
                // Sickly fungal green — a parasited insect.
                color: Color::srgb(0.45, 0.75, 0.35),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world.extend(UNIT_Z)),
        ));
    }
}

pub fn despawn_unit_sprites(
    mut commands: Commands,
    mut removed: RemovedComponents<Unit>,
    sprites: Query<(Entity, &UnitSprite)>,
) {
    let gone: std::collections::HashSet<Entity> = removed.read().collect();
    if gone.is_empty() {
        return;
    }
    for (sprite_e, link) in &sprites {
        if gone.contains(&link.0) {
            commands.entity(sprite_e).despawn();
        }
    }
}

const SELECTION_RING_Z: f32 = 2.4;

/// Per-frame: place each unit sprite by interpolating between its `GridPos`
/// and the next path hex by `edge_progress`. The small unit sprite physically
/// travels hex-centre to hex-centre, visibly crossing each hex it traverses.
pub fn unit_position_system(
    layout: Res<HexLayout>,
    units: Query<(&GridPos, &UnitMovement)>,
    mut sprites: Query<(&UnitSprite, &mut Transform)>,
) {
    for (link, mut transform) in &mut sprites {
        let Ok((gpos, movement)) = units.get(link.0) else {
            continue;
        };
        let from = layout.hex_to_world_pos(gpos.0);
        let world = match movement.path.first() {
            Some(&next) => from.lerp(layout.hex_to_world_pos(next), movement.edge_progress),
            None => from,
        };
        transform.translation = world.extend(UNIT_Z);
    }
}

#[derive(Component)]
pub struct SelectionRing;

/// Spawn/move/despawn a ring sprite that follows `SelectedUnit`.
pub fn selection_ring_system(
    mut commands: Commands,
    selected: Res<SelectedUnit>,
    layout: Res<HexLayout>,
    units: Query<(&GridPos, &UnitMovement)>,
    rings: Query<Entity, With<SelectionRing>>,
) {
    let target = selected.0.and_then(|e| units.get(e).ok());
    match (target, rings.iter().next()) {
        (None, Some(ring)) => commands.entity(ring).despawn(),
        (Some((gpos, movement)), existing) => {
            let from = layout.hex_to_world_pos(gpos.0);
            let world = match movement.path.first() {
                Some(&next) => from.lerp(layout.hex_to_world_pos(next), movement.edge_progress),
                None => from,
            };
            // Ring hugs the small unit body, not the hex.
            let size = organism_sprite_size(&layout) * UNIT_SPRITE_FRACTION * 1.6;
            let ring = existing.unwrap_or_else(|| commands.spawn(SelectionRing).id());
            commands.entity(ring).insert((
                Sprite {
                    color: Color::srgba(1.0, 1.0, 0.4, 0.7),
                    custom_size: Some(size),
                    ..default()
                },
                Transform::from_translation(world.extend(SELECTION_RING_Z)),
            ));
        }
        (None, None) => {}
    }
}
