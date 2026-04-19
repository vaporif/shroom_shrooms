use bevy::prelude::*;
use shroom_core::*;

const TILE_SIZE: f32 = 16.0;

#[derive(Component)]
pub struct TerrainSprite;

pub fn terrain_render_system(
    mut commands: Commands,
    tiles: Query<(&GridPos, &Tile), Changed<Tile>>,
) {
    for (gpos, tile) in tiles.iter() {
        let color = match tile.terrain {
            TerrainType::Soil => Color::srgb(0.45, 0.32, 0.18),
            TerrainType::Rock => Color::srgb(0.5, 0.5, 0.5),
            TerrainType::Water => Color::srgb(0.2, 0.4, 0.8),
            TerrainType::Root => Color::srgb(0.3, 0.5, 0.2),
            TerrainType::Ruin => Color::srgb(0.6, 0.55, 0.4),
            TerrainType::Toxic => Color::srgb(0.5, 0.8, 0.1),
            TerrainType::Surface => Color::srgb(0.3, 0.6, 0.3),
        };

        let world_pos = Vec3::new(
            gpos.0.x as f32 * TILE_SIZE,
            gpos.0.y as f32 * TILE_SIZE,
            0.0,
        );

        commands.spawn((
            TerrainSprite,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            Transform::from_translation(world_pos),
        ));
    }
}
