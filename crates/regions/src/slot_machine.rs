use bevy::ecs::message::{Message, MessageReader, MessageWriter};
use bevy::prelude::*;
use kingdom_core::{DecompositionComplete, UnlockOption, UnlockPool};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;

#[derive(Message)]
pub struct SlotMachineTriggered {
    pub options: Vec<UnlockOption>,
}

#[derive(Resource)]
pub struct SlotMachineRng(pub StdRng);

impl Default for SlotMachineRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(7))
    }
}

pub fn slot_machine_system(
    mut decomp_messages: MessageReader<DecompositionComplete>,
    mut slot_messages: MessageWriter<SlotMachineTriggered>,
    mut rng: ResMut<SlotMachineRng>,
) {
    for event in decomp_messages.read() {
        if !event.was_unique {
            continue;
        }
        let pool_options = unlock_pool_options(UnlockPool::Decomposition);
        let selected: Vec<UnlockOption> = pool_options.sample(&mut rng.0, 3).cloned().collect();
        slot_messages.write(SlotMachineTriggered { options: selected });
    }
}

fn unlock_pool_options(pool: UnlockPool) -> Vec<UnlockOption> {
    match pool {
        UnlockPool::Organic => vec![
            UnlockOption {
                name: "Rapid Growth".into(),
                description: "Tips move 25% faster".into(),
                pool,
            },
            UnlockOption {
                name: "Regeneration".into(),
                description: "Biomass recovers 10% faster".into(),
                pool,
            },
            UnlockOption {
                name: "Efficient Digestion".into(),
                description: "Decomposers extract 50% more nutrients".into(),
                pool,
            },
            UnlockOption {
                name: "Wide Branching".into(),
                description: "Branch probability +10%".into(),
                pool,
            },
        ],
        UnlockPool::Mineral => vec![
            UnlockOption {
                name: "Hardened Hyphae".into(),
                description: "Biomass decay slowed by 30%".into(),
                pool,
            },
            UnlockOption {
                name: "Rock Tolerance".into(),
                description: "Tips can enter Rock tiles (slowly)".into(),
                pool,
            },
            UnlockOption {
                name: "Mineral Armor".into(),
                description: "Border friction requires 2x ratio to flip".into(),
                pool,
            },
            UnlockOption {
                name: "Crystal Resonance".into(),
                description: "Fragment detection radius +5 tiles".into(),
                pool,
            },
        ],
        UnlockPool::Ruins => vec![
            UnlockOption {
                name: "Ancient Memory".into(),
                description: "All specializations invest 25% faster".into(),
                pool,
            },
            UnlockOption {
                name: "Ruin Network".into(),
                description: "Tiles in ruins provide +0.3 nutrients".into(),
                pool,
            },
            UnlockOption {
                name: "Lost Technology".into(),
                description: "New ability: Pulse Scan (reveal 10-tile radius)".into(),
                pool,
            },
            UnlockOption {
                name: "Fragment Echo".into(),
                description: "Nearest unfused fragment shown on map".into(),
                pool,
            },
        ],
        UnlockPool::Decomposition => vec![
            UnlockOption {
                name: "Rich Bloom".into(),
                description: "Decomposition leaves richer soil behind".into(),
                pool,
            },
            UnlockOption {
                name: "Sweet Trade".into(),
                description: "Symbiosis with plants yields more sugars".into(),
                pool,
            },
            UnlockOption {
                name: "Eager Flow".into(),
                description: "Mycelium pushes a little harder past the frontier".into(),
                pool,
            },
            UnlockOption {
                name: "Hardened Hyphae".into(),
                description: "Dry tiles cling to the network for longer before dying back".into(),
                pool,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use kingdom_core::Hex;

    use super::*;

    #[test]
    fn slot_machine_fires_on_unique_decomp() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(0));
        let captured_c = captured.clone();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SlotMachineRng>();
        app.add_message::<DecompositionComplete>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(
            Update,
            (
                slot_machine_system,
                (move |mut r: MessageReader<SlotMachineTriggered>| {
                    for ev in r.read() {
                        if ev.options.len() == 3 {
                            *captured_c.lock().unwrap() += 1;
                        }
                    }
                }),
            )
                .chain(),
        );
        app.world_mut().write_message(DecompositionComplete {
            pos: Hex::ZERO,
            was_unique: true,
        });
        app.update();
        assert_eq!(*captured.lock().unwrap(), 1);
    }

    #[test]
    fn slot_machine_quiet_on_organic_decomp() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(0));
        let captured_c = captured.clone();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SlotMachineRng>();
        app.add_message::<DecompositionComplete>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(
            Update,
            (
                slot_machine_system,
                (move |mut r: MessageReader<SlotMachineTriggered>| {
                    for _ in r.read() {
                        *captured_c.lock().unwrap() += 1;
                    }
                }),
            )
                .chain(),
        );
        app.world_mut().write_message(DecompositionComplete {
            pos: Hex::ZERO,
            was_unique: false,
        });
        app.update();
        assert_eq!(*captured.lock().unwrap(), 0);
    }
}
