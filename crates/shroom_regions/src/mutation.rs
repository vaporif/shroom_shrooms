use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use shroom_core::{SlotMachineTriggered, UnlockOption};

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct AppliedMutations {
    pub mutations: Vec<UnlockOption>,
}

/// Stub: auto-selects first option. Will be driven by player UI.
pub fn mutation_system(
    mut slot_messages: MessageReader<SlotMachineTriggered>,
    mut mutations: ResMut<AppliedMutations>,
) {
    for event in slot_messages.read() {
        if let Some(first) = event.options.first() {
            mutations.mutations.push(first.clone());
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
}
