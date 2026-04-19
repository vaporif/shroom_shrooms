use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use shroom_core::{MutationSelection, SlotMachineTriggered, UnlockOption};

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct AppliedMutations {
    pub mutations: Vec<UnlockOption>,
}

/// Picks the player-selected option from the slot machine, falling back to the first.
pub fn mutation_system(
    mut slot_messages: MessageReader<SlotMachineTriggered>,
    mut mutations: ResMut<AppliedMutations>,
    mut selection: ResMut<MutationSelection>,
) {
    for event in slot_messages.read() {
        let index = selection.selected_index.take().unwrap_or(0);
        if let Some(chosen) = event.options.get(index).or_else(|| event.options.first()) {
            mutations.mutations.push(chosen.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::UnlockPool;

    #[test]
    fn mutation_applied_from_slot_machine_event() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<AppliedMutations>();
        app.init_resource::<MutationSelection>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(Update, mutation_system);

        app.world_mut().write_message(SlotMachineTriggered {
            pool: UnlockPool::Organic,
            options: vec![UnlockOption {
                name: "Test Mutation".into(),
                description: "test".into(),
                pool: UnlockPool::Organic,
            }],
        });

        app.update();

        let mutations = app.world().resource::<AppliedMutations>();
        assert_eq!(mutations.mutations.len(), 1);
        assert_eq!(mutations.mutations[0].name, "Test Mutation");
    }

    #[test]
    fn mutation_uses_selected_index() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<AppliedMutations>();
        app.init_resource::<MutationSelection>();
        app.add_message::<SlotMachineTriggered>();
        app.add_systems(Update, mutation_system);

        app.world_mut()
            .resource_mut::<MutationSelection>()
            .selected_index = Some(1);

        app.world_mut().write_message(SlotMachineTriggered {
            pool: UnlockPool::Organic,
            options: vec![
                UnlockOption {
                    name: "First".into(),
                    description: "first".into(),
                    pool: UnlockPool::Organic,
                },
                UnlockOption {
                    name: "Second".into(),
                    description: "second".into(),
                    pool: UnlockPool::Organic,
                },
            ],
        });

        app.update();

        let mutations = app.world().resource::<AppliedMutations>();
        assert_eq!(mutations.mutations.len(), 1);
        assert_eq!(mutations.mutations[0].name, "Second");

        // Selection should be consumed
        let sel = app.world().resource::<MutationSelection>();
        assert!(sel.selected_index.is_none());
    }
}
