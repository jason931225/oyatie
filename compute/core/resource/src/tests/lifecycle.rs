use crate::ResourceState;

#[test]
fn resource_state_as_str_and_parse_round_trip() {
    let variants = [
        ResourceState::Pending,
        ResourceState::Running,
        ResourceState::Stopped,
        ResourceState::Terminated,
        ResourceState::Error,
    ];
    for state in variants {
        assert_eq!(
            ResourceState::parse(state.as_str()),
            Some(state),
            "parse(as_str({state:?})) must round-trip"
        );
    }
}

#[test]
fn resource_state_parse_rejects_unknown_inputs() {
    assert_eq!(ResourceState::parse("bogus"), None);
    assert_eq!(ResourceState::parse(""), None);
    assert_eq!(ResourceState::parse("RUNNING"), None);
    assert_eq!(ResourceState::parse("Pending"), None);
}

#[test]
fn resource_state_classifiers_match_only_correct_variant() {
    assert!(ResourceState::Running.is_active());
    assert!(!ResourceState::Pending.is_active());
    assert!(!ResourceState::Stopped.is_active());
    assert!(!ResourceState::Terminated.is_active());
    assert!(!ResourceState::Error.is_active());

    assert!(ResourceState::Stopped.is_quiescent());
    assert!(!ResourceState::Pending.is_quiescent());
    assert!(!ResourceState::Running.is_quiescent());
    assert!(!ResourceState::Terminated.is_quiescent());
    assert!(!ResourceState::Error.is_quiescent());
}

#[test]
fn terminated_allowed_next_contains_only_self_loop() {
    let nexts = ResourceState::Terminated.allowed_next();
    assert_eq!(nexts, &[ResourceState::Terminated]);
}

#[test]
fn transition_graph_admits_exactly_the_legal_pairs() {
    let all_states = [
        ResourceState::Pending,
        ResourceState::Running,
        ResourceState::Stopped,
        ResourceState::Terminated,
        ResourceState::Error,
    ];
    let legal = [
        (ResourceState::Pending, ResourceState::Running),
        (ResourceState::Pending, ResourceState::Error),
        (ResourceState::Pending, ResourceState::Terminated),
        (ResourceState::Running, ResourceState::Stopped),
        (ResourceState::Running, ResourceState::Error),
        (ResourceState::Running, ResourceState::Terminated),
        (ResourceState::Stopped, ResourceState::Running),
        (ResourceState::Stopped, ResourceState::Error),
        (ResourceState::Stopped, ResourceState::Terminated),
        (ResourceState::Error, ResourceState::Terminated),
    ];
    for &from in &all_states {
        for &to in &all_states {
            let expected = from == to || legal.contains(&(from, to));
            assert_eq!(
                from.can_transition_to(to),
                expected,
                "can_transition_to({from:?}, {to:?}) must be {expected}"
            );
        }
    }
}
