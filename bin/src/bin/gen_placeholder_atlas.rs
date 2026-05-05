//! Placeholder atlas generator. Run with:
//!   cargo run -p fungai --bin gen_placeholder_atlas --features gen-atlas
//!
//! Writes 1 column x 7 rows of 49x56 pointy-top hex sprites to
//! bin/assets/sprites/terrain/terrain_atlas.png. Each cell is filled with
//! its TerrainType base color plus a tiny dither so it does not read flat;
//! pixels outside the hex polygon are transparent.

#![cfg(feature = "gen-atlas")]

use std::path::PathBuf;

use fungai_core::TerrainType;
use fungai_render::terrain_base_color;
use image::{Rgba, RgbaImage};

const TILE_W: u32 = 49;
const TILE_H: u32 = 56;
const ROWS: u32 = 7;

const TERRAINS: [TerrainType; 7] = [
    TerrainType::Soil,
    TerrainType::Rock,
    TerrainType::Water,
    TerrainType::Root,
    TerrainType::Ruin,
    TerrainType::Toxic,
    TerrainType::Surface,
];

fn srgb_byte(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

fn point_in_pointy_hex(px: f32, py: f32, cx: f32, cy: f32, half_w: f32, half_h: f32) -> bool {
    let dx = (px - cx).abs();
    let dy = (py - cy).abs();
    if dx > half_w || dy > half_h {
        return false;
    }
    // Pointy-top hex: clip the four diagonal corners.
    let slope = half_h / half_w * 2.0;
    dx * slope + dy <= half_h * 2.0
}

fn main() {
    let mut img = RgbaImage::new(TILE_W, TILE_H * ROWS);
    let cx = (TILE_W as f32 - 1.0) * 0.5;
    let half_w = (TILE_W as f32 - 1.0) * 0.5;
    let half_h = (TILE_H as f32 - 1.0) * 0.5;

    for (row, terrain) in TERRAINS.iter().copied().enumerate() {
        let base = terrain_base_color(terrain);
        let row_offset = row as u32 * TILE_H;
        let cy_local = (TILE_H as f32 - 1.0) * 0.5;

        for y in 0..TILE_H {
            for x in 0..TILE_W {
                if !point_in_pointy_hex(x as f32, y as f32, cx, cy_local, half_w, half_h) {
                    img.put_pixel(x, row_offset + y, Rgba([0, 0, 0, 0]));
                    continue;
                }

                // Cheap deterministic dither: +/- 6% lightness on a 5x5 hash.
                let hash = (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)) % 5;
                let dither = match hash {
                    0 => 1.06,
                    1 => 0.94,
                    _ => 1.0,
                };

                img.put_pixel(
                    x,
                    row_offset + y,
                    Rgba([
                        srgb_byte(base.red * dither),
                        srgb_byte(base.green * dither),
                        srgb_byte(base.blue * dither),
                        255,
                    ]),
                );
            }
        }
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("assets/sprites/terrain");
    std::fs::create_dir_all(&path).expect("create assets/sprites/terrain");
    path.push("terrain_atlas.png");
    img.save(&path).expect("write terrain_atlas.png");
    println!("wrote {}", path.display());
}
