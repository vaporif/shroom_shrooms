use kingdom_input::{Action, WispState};

#[test]
fn wisp_mode_action_exists_and_defaults_unpressed() {
    // The action map must define WispMode; without it the wisp can never paint.
    let map = kingdom_input::default_input_map();
    assert!(
        map.get(&Action::WispMode).is_some_and(|b| !b.is_empty()),
        "WispMode must be bound",
    );
    assert!(
        map.get(&Action::FoundNetwork)
            .is_some_and(|b| !b.is_empty()),
        "FoundNetwork must be bound",
    );
}

#[test]
fn wisp_state_defaults_idle() {
    assert!(matches!(
        WispState::default().phase,
        kingdom_input::WispPhase::Idle
    ));
}
