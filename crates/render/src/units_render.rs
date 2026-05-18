use bevy::prelude::*;
use kingdom_core::{GridPos, HexLayout, Hive};

use crate::assets::EntitySprites;
use crate::entity_render::organism_sprite_size;

/// Z layer for the unit-layer sprites, above terrain/network.
const HIVE_Z: f32 = 1.5;

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
