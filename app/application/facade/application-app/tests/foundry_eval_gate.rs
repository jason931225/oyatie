#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AutonomyTier, CapabilityAction, CapabilityRegistration, DataClass, Foundation, FoundationError,
};

#[test]
fn foundation_capability_publish_requires_passing_eval_gate() {
    let mut foundation = Foundation::default();
    let registration = CapabilityRegistration {
        capability_id: "cap.demo.eval-gated".into(),
        namespace: "demo".into(),
        action: CapabilityAction::Other,
        required_tier: AutonomyTier::T1ViewOnly,
        touched_privacy_data_classes: application_app::privacy_data_classes_from(&[
            DataClass::InternalOnly,
        ])
        .unwrap(),
        evidence_topic: "oya.foundry.capability.invoked".into(),
    };

    assert_eq!(
        foundation.register_capability(registration.clone()),
        Err(FoundationError::CapabilityEvalGateNotReady)
    );

    support::seed_passing_eval(&mut foundation, "cap.demo.eval-gated");
    let capability = foundation
        .register_capability(registration)
        .expect("passing eval gate permits capability publish");
    assert_eq!(capability.id, "cap.demo.eval-gated");
}
