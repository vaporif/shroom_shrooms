use std::collections::HashSet;

use bevy::ecs::message::{Message, MessageWriter};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    BIAS_MAGNITUDE_CAP, BIAS_STROKE_INTENSITY, DRAG_THRESHOLD_PX, GamePhase, GridPos, GridWorld,
    Hex, HexLayout, SAMPLE_HEX_DISTANCE, SAMPLE_INTERVAL_SECS, TAP_TIME_SECS, Tile,
    WISP_SENSE_RADIUS_HEX,
};
use leafwing_input_manager::prelude::*;

use crate::action::Action;
use crate::camera::GameCamera;

#[derive(Default, Clone, Copy, Debug)]
pub enum WispPhase {
    #[default]
    Idle,
    Primed {
        start_pos: Vec2,
        start_time: f32,
    },
    Stroking {
        last_sample_pos: Vec2,
        last_sample_time: f32,
    },
}

#[derive(Resource, Default)]
pub struct WispState {
    pub phase: WispPhase,
    owned: HashSet<Hex>,
}

#[derive(Message)]
pub struct TileTapped {
    pub pos: Hex,
}

#[derive(SystemParam)]
pub struct WispInput<'w, 's> {
    actions: Res<'w, ActionState<Action>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
    ui_interactions: Query<'w, 's, &'static Interaction, With<Button>>,
}

impl WispInput<'_, '_> {
    fn cursor_world(&self) -> Option<Vec2> {
        let window = self.windows.single().ok()?;
        let cursor = window.cursor_position()?;
        let (camera, cam_xform) = self.cameras.single().ok()?;
        camera.viewport_to_world_2d(cam_xform, cursor).ok()
    }

    fn ui_blocking(&self) -> bool {
        self.ui_interactions
            .iter()
            .any(|i| !matches!(i, Interaction::None))
    }
}

#[derive(SystemParam)]
pub struct WispWorld<'w, 's> {
    layout: Res<'w, HexLayout>,
    grid: Res<'w, GridWorld>,
    tiles: Query<'w, 's, (&'static GridPos, &'static mut Tile)>,
}

impl WispWorld<'_, '_> {
    fn refresh_owned(&self, owned: &mut HashSet<Hex>) {
        owned.clear();
        owned.extend(
            self.tiles
                .iter()
                .filter_map(|(gp, t)| t.region_id.is_some().then_some(gp.0)),
        );
    }

    fn write_segment(&mut self, p1: Vec2, p2: Vec2, owned: &HashSet<Hex>) {
        let direction = (p2 - p1).normalize_or_zero();
        if direction == Vec2::ZERO {
            return;
        }
        let hex = self.layout.world_pos_to_hex(p2);
        let Some(&entity) = self.grid.tiles.get(&hex) else {
            return;
        };
        let falloff = network_proximity_factor(hex, owned);
        if falloff <= 0.0 {
            return;
        }
        let Ok((_, mut tile)) = self.tiles.get_mut(entity) else {
            return;
        };
        let candidate = tile.priority_bias + direction * BIAS_STROKE_INTENSITY * falloff;
        let mag = candidate.length();
        let new_bias = if mag > BIAS_MAGNITUDE_CAP {
            candidate * (BIAS_MAGNITUDE_CAP / mag)
        } else {
            candidate
        };
        if tile.priority_bias != new_bias {
            tile.priority_bias = new_bias;
        }
    }
}

pub fn wisp_input_system(
    input: WispInput,
    time: Res<Time>,
    phase: Res<GamePhase>,
    mut world: WispWorld,
    mut wisp: ResMut<WispState>,
    mut taps: MessageWriter<TileTapped>,
) {
    if *phase != GamePhase::Playing {
        wisp.phase = WispPhase::Idle;
        return;
    }
    if input.ui_blocking() {
        return;
    }

    let Some(cursor_world) = input.cursor_world() else {
        return;
    };
    let now = time.elapsed_secs();

    let pressed = input.actions.pressed(&Action::Paint);
    let just_pressed = input.actions.just_pressed(&Action::Paint);
    let just_released = input.actions.just_released(&Action::Paint);

    if !pressed && !just_pressed && !just_released {
        return;
    }

    wisp.phase = match wisp.phase {
        WispPhase::Idle => {
            if just_pressed {
                WispPhase::Primed {
                    start_pos: cursor_world,
                    start_time: now,
                }
            } else {
                WispPhase::Idle
            }
        }
        WispPhase::Primed {
            start_pos,
            start_time,
        } => {
            if just_released {
                if cursor_world.distance(start_pos) < DRAG_THRESHOLD_PX
                    && now - start_time < TAP_TIME_SECS
                {
                    let hex = world.layout.world_pos_to_hex(start_pos);
                    taps.write(TileTapped { pos: hex });
                }
                WispPhase::Idle
            } else if pressed && cursor_world.distance(start_pos) > DRAG_THRESHOLD_PX {
                world.refresh_owned(&mut wisp.owned);
                world.write_segment(start_pos, cursor_world, &wisp.owned);
                WispPhase::Stroking {
                    last_sample_pos: cursor_world,
                    last_sample_time: now,
                }
            } else {
                WispPhase::Primed {
                    start_pos,
                    start_time,
                }
            }
        }
        WispPhase::Stroking {
            last_sample_pos,
            last_sample_time,
        } => {
            if just_released {
                WispPhase::Idle
            } else if pressed {
                let elapsed = now - last_sample_time;
                let hex_size = world.layout.scale.x;
                if elapsed > SAMPLE_INTERVAL_SECS
                    || cursor_world.distance(last_sample_pos) > SAMPLE_HEX_DISTANCE * hex_size
                {
                    world.refresh_owned(&mut wisp.owned);
                    world.write_segment(last_sample_pos, cursor_world, &wisp.owned);
                    WispPhase::Stroking {
                        last_sample_pos: cursor_world,
                        last_sample_time: now,
                    }
                } else {
                    WispPhase::Stroking {
                        last_sample_pos,
                        last_sample_time,
                    }
                }
            } else {
                WispPhase::Idle
            }
        }
    };
}

fn network_proximity_factor(hex: Hex, owned: &HashSet<Hex>) -> f32 {
    if owned.is_empty() {
        return 0.0;
    }
    let radius = WISP_SENSE_RADIUS_HEX;
    let nearest = owned
        .iter()
        .map(|o| hex.unsigned_distance_to(*o))
        .min()
        .unwrap_or(u32::MAX);
    if nearest > radius {
        return 0.0;
    }
    1.0 - (nearest as f32) / (radius as f32 + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wisp_state_default_is_idle() {
        let s = WispState::default();
        assert!(matches!(s.phase, WispPhase::Idle));
    }
}
