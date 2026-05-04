use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use fungai_core::{SlotMachineTriggered, StudyComplete, UnlockOption, UnlockPool};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;

#[derive(Resource)]
pub struct SlotMachineRng(pub StdRng);

impl Default for SlotMachineRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(7))
    }
}

pub fn slot_machine_system(
    mut study_messages: MessageReader<StudyComplete>,
    mut slot_messages: MessageWriter<SlotMachineTriggered>,
    mut rng: ResMut<SlotMachineRng>,
) {
    for event in study_messages.read() {
        let pool_options = unlock_pool_options(event.pool);
        let selected: Vec<UnlockOption> = pool_options.sample(&mut rng.0, 3).cloned().collect();

        slot_messages.write(SlotMachineTriggered {
            pool: event.pool,
            options: selected,
        });
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
                name: "Enzyme Burst+".into(),
                description: "Enzyme burst radius doubled".into(),
                pool,
            },
            UnlockOption {
                name: "Toxin Resistance".into(),
                description: "Tips survive in Toxic terrain".into(),
                pool,
            },
            UnlockOption {
                name: "Nutrient Conversion".into(),
                description: "Convert energy to nutrients at 2:1".into(),
                pool,
            },
            UnlockOption {
                name: "Acid Secretion".into(),
                description: "Dissolve Rock tiles adjacent to decomposer regions".into(),
                pool,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use fungai_core::Hex;

    use super::*;

    #[test]
    fn slot_machine_produces_three_options() {
        let capture = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let capture_clone = capture.clone();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SlotMachineRng>();
        app.add_message::<StudyComplete>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(
            Update,
            (
                slot_machine_system,
                move |mut reader: MessageReader<SlotMachineTriggered>| {
                    for msg in reader.read() {
                        capture_clone
                            .lock()
                            .unwrap()
                            .push((msg.pool, msg.options.len()));
                    }
                },
            )
                .chain(),
        );

        app.world_mut().write_message(StudyComplete {
            pos: Hex::ZERO,
            pool: UnlockPool::Organic,
        });

        app.update();

        let results = capture.lock().unwrap();
        assert_eq!(
            results.len(),
            1,
            "should have received one SlotMachineTriggered"
        );
        assert_eq!(results[0].0, UnlockPool::Organic);
        assert_eq!(results[0].1, 3, "should have 3 options");
    }
}
